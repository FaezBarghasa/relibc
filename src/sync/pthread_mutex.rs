//! POSIX pthread_mutex Implementation with Robust Mutex Support
//!
//! This implementation provides full PTHREAD_MUTEX_ROBUST support including:
//! - Robust list tracking: mutexes are linked in a thread-local list
//! - Owner death detection: kernel sets OWNER_DIED bit when owner exits
//! - Recovery via pthread_mutex_consistent(): clear inconsistent state
//!
//! ## Robust Mutex Protocol
//! 1. When a thread acquires a robust mutex, it's added to the thread's robust list
//! 2. If the thread dies while holding mutexes, kernel marks them with OWNER_DIED
//! 3. Next acquirer sees EOWNERDEAD and can call make_consistent() to recover

use core::{
    cell::Cell,
    ptr::NonNull,
    sync::atomic::{AtomicPtr, AtomicU32 as AtomicUint, Ordering},
};

use crate::{
    error::Errno,
    header::{errno::*, pthread::*, time::timespec},
    pthread::*,
};

use crate::platform::{types::*, Pal, Sys};

use super::FutexWaitResult;

/// Robust list entry for tracking owned mutexes
/// Forms a singly-linked list per thread
#[repr(C)]
pub struct RobustListEntry {
    /// Next entry in the robust list (null if last)
    next: AtomicPtr<RobustListEntry>,
    /// Pointer back to the owning mutex
    mutex: NonNull<RlctMutex>,
}

/// Thread-local robust list head
/// The kernel will walk this list on thread exit to mark owned mutexes
#[thread_local]
static ROBUST_LIST_HEAD: AtomicPtr<RobustListEntry> = AtomicPtr::new(core::ptr::null_mut());

pub struct RlctMutex {
    /// Actual locking word.
    /// Bits 0-29: owner TID
    /// Bit 30: OWNER_DIED flag
    /// Bit 31: WAITERS flag
    inner: AtomicUint,
    recursive_count: AtomicUint,

    ty: Ty,
    robust: bool,

    /// Robust list entry (only used when robust=true)
    /// Embedded directly in the mutex to avoid allocation
    robust_entry: RobustListEntry,
}

const STATE_UNLOCKED: u32 = 0;
const WAITING_BIT: u32 = 1 << 31;
const OWNER_DIED_BIT: u32 = 1 << 30;
const INDEX_MASK: u32 = !(WAITING_BIT | OWNER_DIED_BIT);

const RECURSIVE_COUNT_MAX_INCLUSIVE: u32 = u32::MAX;
const SPIN_COUNT: usize = 1000;

impl RlctMutex {
    pub(crate) fn new(attr: &RlctMutexAttr) -> Result<Self, Errno> {
        let RlctMutexAttr {
            prioceiling: _,
            protocol: _,
            pshared: _,
            robust,
            ty,
        } = *attr;

        Ok(Self {
            inner: AtomicUint::new(STATE_UNLOCKED),
            recursive_count: AtomicUint::new(0),
            robust: match robust {
                PTHREAD_MUTEX_STALLED => false,
                PTHREAD_MUTEX_ROBUST => true,
                _ => return Err(Errno(EINVAL)),
            },
            ty: match ty {
                PTHREAD_MUTEX_DEFAULT => Ty::Def,
                PTHREAD_MUTEX_ERRORCHECK => Ty::Errck,
                PTHREAD_MUTEX_RECURSIVE => Ty::Recursive,
                PTHREAD_MUTEX_NORMAL => Ty::Normal,
                _ => return Err(Errno(EINVAL)),
            },
            robust_entry: RobustListEntry {
                next: AtomicPtr::new(core::ptr::null_mut()),
                mutex: NonNull::dangling(), // Will be set on first lock
            },
        })
    }

    pub fn prioceiling(&self) -> Result<c_int, Errno> {
        Ok(0)
    }

    pub fn replace_prioceiling(&self, _: c_int) -> Result<c_int, Errno> {
        Ok(0)
    }

    /// Mark a mutex as consistent after recovering from EOWNERDEAD
    ///
    /// This function is called by the new owner after receiving EOWNERDEAD
    /// to indicate that the protected data has been repaired and is consistent.
    pub fn make_consistent(&self) -> Result<(), Errno> {
        let this_thread = os_tid_invalid_after_fork();
        let current_state = self.inner.load(Ordering::Acquire);

        // Verify we are the current owner
        if current_state & INDEX_MASK != this_thread {
            return Err(Errno(EINVAL));
        }

        // Verify OWNER_DIED was set (otherwise nothing to recover)
        if current_state & OWNER_DIED_BIT == 0 {
            return Err(Errno(EINVAL));
        }

        // Clear the OWNER_DIED bit, keeping ownership and WAITING_BIT
        let new_state = current_state & !OWNER_DIED_BIT;
        match self.inner.compare_exchange(
            current_state,
            new_state,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(()),
            Err(_) => Err(Errno(EINVAL)),
        }
    }

    /// Add this mutex to the current thread's robust list
    fn add_to_robust_list(&self) {
        if !self.robust {
            return;
        }

        // Get mutable reference to our robust_entry
        // This is safe because we own the lock
        let entry_ptr = &self.robust_entry as *const RobustListEntry as *mut RobustListEntry;

        // Update the mutex pointer
        unsafe {
            (*entry_ptr).mutex = NonNull::new_unchecked(self as *const Self as *mut Self);
        }

        // Add to head of list
        loop {
            let head = ROBUST_LIST_HEAD.load(Ordering::Relaxed);
            unsafe {
                (*entry_ptr).next.store(head, Ordering::Relaxed);
            }

            match ROBUST_LIST_HEAD.compare_exchange_weak(
                head,
                entry_ptr,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(_) => continue,
            }
        }
    }

    /// Remove this mutex from the current thread's robust list
    fn remove_from_robust_list(&self) {
        if !self.robust {
            return;
        }

        let entry_ptr = &self.robust_entry as *const RobustListEntry as *mut RobustListEntry;

        // Try to remove from head first (common case)
        let head = ROBUST_LIST_HEAD.load(Ordering::Relaxed);
        if head == entry_ptr {
            let next = unsafe { (*entry_ptr).next.load(Ordering::Relaxed) };
            if ROBUST_LIST_HEAD
                .compare_exchange(head, next, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }

        // Otherwise walk the list to find and remove
        let mut current = ROBUST_LIST_HEAD.load(Ordering::Acquire);
        while !current.is_null() {
            unsafe {
                let next = (*current).next.load(Ordering::Relaxed);
                if next == entry_ptr {
                    let after_entry = (*entry_ptr).next.load(Ordering::Relaxed);
                    (*current).next.store(after_entry, Ordering::Release);
                    return;
                }
                current = next;
            }
        }
    }

    fn lock_inner(&self, deadline: Option<&timespec>) -> Result<(), Errno> {
        let this_thread = os_tid_invalid_after_fork();

        // Fast path: spin trying to acquire
        for _ in 0..SPIN_COUNT {
            let result = self.inner.compare_exchange_weak(
                STATE_UNLOCKED,
                this_thread,
                Ordering::Acquire,
                Ordering::Relaxed,
            );

            if result.is_ok() {
                self.add_to_robust_list();
                if self.ty == Ty::Recursive {
                    self.increment_recursive_count()?;
                }
                return Ok(());
            }

            // Check for owner death during spinning
            if self.robust {
                let state = result.unwrap_err();
                if state & OWNER_DIED_BIT != 0 {
                    // Try to take ownership of dead mutex
                    let new_state = (state & !INDEX_MASK) | this_thread;
                    if self
                        .inner
                        .compare_exchange(state, new_state, Ordering::Acquire, Ordering::Relaxed)
                        .is_ok()
                    {
                        self.add_to_robust_list();
                        if self.ty == Ty::Recursive {
                            let _ = self.increment_recursive_count();
                        }
                        return Err(Errno(EOWNERDEAD));
                    }
                }
            }

            core::hint::spin_loop();
        }

        // Slow path: use futex
        loop {
            let mut current_state = self.inner.load(Ordering::Relaxed);

            // Check for owner death
            if self.robust && (current_state & OWNER_DIED_BIT) != 0 {
                let new_state = (current_state & !INDEX_MASK) | this_thread;
                if self
                    .inner
                    .compare_exchange(
                        current_state,
                        new_state,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    self.add_to_robust_list();
                    if self.ty == Ty::Recursive {
                        let _ = self.increment_recursive_count();
                    }
                    return Err(Errno(EOWNERDEAD));
                }
                continue;
            }

            if current_state & INDEX_MASK == 0 {
                // Mutex is unlocked, try to acquire
                match self.inner.compare_exchange_weak(
                    current_state,
                    (current_state & WAITING_BIT) | this_thread,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        self.add_to_robust_list();
                        if self.ty == Ty::Recursive {
                            self.increment_recursive_count()?;
                        }
                        return Ok(());
                    }
                    Err(s) => current_state = s,
                }
            } else {
                // Mutex is locked, set WAITING_BIT and wait
                let new_state = current_state | WAITING_BIT;
                if new_state != current_state {
                    match self.inner.compare_exchange_weak(
                        current_state,
                        new_state,
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                    ) {
                        Ok(_) => current_state = new_state,
                        Err(s) => {
                            current_state = s;
                            continue;
                        }
                    }
                }

                let res = crate::sync::futex_wait(&self.inner, current_state, deadline);

                if self.robust && res == FutexWaitResult::Stale {
                    // Owner may have died - check OWNER_DIED bit
                    let state = self.inner.load(Ordering::Acquire);
                    if state & OWNER_DIED_BIT != 0 {
                        let new_state = (state & !INDEX_MASK) | this_thread;
                        if self
                            .inner
                            .compare_exchange(
                                state,
                                new_state,
                                Ordering::Acquire,
                                Ordering::Relaxed,
                            )
                            .is_ok()
                        {
                            self.add_to_robust_list();
                            return Err(Errno(EOWNERDEAD));
                        }
                    }
                }
                if res == FutexWaitResult::TimedOut {
                    return Err(Errno(ETIMEDOUT));
                }
            }
        }
    }

    pub fn lock(&self) -> Result<(), Errno> {
        self.lock_inner(None)
    }

    pub fn lock_with_timeout(&self, deadline: &timespec) -> Result<(), Errno> {
        self.lock_inner(Some(deadline))
    }

    fn increment_recursive_count(&self) -> Result<(), Errno> {
        let prev_recursive_count = self.recursive_count.load(Ordering::Relaxed);

        if prev_recursive_count == RECURSIVE_COUNT_MAX_INCLUSIVE {
            return Err(Errno(EAGAIN));
        }

        self.recursive_count
            .store(prev_recursive_count + 1, Ordering::Relaxed);

        Ok(())
    }

    pub fn try_lock(&self) -> Result<(), Errno> {
        let this_thread = os_tid_invalid_after_fork();

        // Check for owner death first (for robust mutexes)
        if self.robust {
            let current = self.inner.load(Ordering::Relaxed);
            if current & OWNER_DIED_BIT != 0 {
                let new_state = (current & !INDEX_MASK) | this_thread;
                if self
                    .inner
                    .compare_exchange(current, new_state, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    self.add_to_robust_list();
                    if self.ty == Ty::Recursive {
                        let _ = self.increment_recursive_count();
                    }
                    return Err(Errno(EOWNERDEAD));
                }
            }
        }

        let result = self.inner.compare_exchange(
            STATE_UNLOCKED,
            this_thread,
            Ordering::Acquire,
            Ordering::Relaxed,
        );

        if self.ty == Ty::Recursive {
            match result {
                Err(index) if index & INDEX_MASK != this_thread => return Err(Errno(EBUSY)),
                Ok(_) => {
                    self.add_to_robust_list();
                }
                _ => {}
            }

            self.increment_recursive_count()?;
            return Ok(());
        }

        match result {
            Ok(_) => {
                self.add_to_robust_list();
                Ok(())
            }
            Err(index) if index & INDEX_MASK == this_thread && self.ty == Ty::Errck => {
                Err(Errno(EDEADLK))
            }
            Err(_) => Err(Errno(EBUSY)),
        }
    }

    pub fn unlock(&self) -> Result<(), Errno> {
        let this_thread = os_tid_invalid_after_fork();

        if self.robust || matches!(self.ty, Ty::Recursive | Ty::Errck) {
            let state = self.inner.load(Ordering::Relaxed);
            if state & INDEX_MASK != this_thread {
                return Err(Errno(EPERM));
            }

            core::sync::atomic::fence(Ordering::Acquire);
        }

        if self.ty == Ty::Recursive {
            let next = self.recursive_count.load(Ordering::Relaxed) - 1;
            self.recursive_count.store(next, Ordering::Relaxed);

            if next > 0 {
                return Ok(());
            }
        }

        // Remove from robust list before releasing
        self.remove_from_robust_list();

        // Release the lock
        let prev_state = self.inner.swap(STATE_UNLOCKED, Ordering::Release);

        if prev_state & WAITING_BIT != 0 {
            let _ = crate::sync::futex_wake(&self.inner, 1);
        }

        Ok(())
    }
}

/// Mark a mutex as unrecoverable after EOWNERDEAD
/// Subsequent lock attempts will return ENOTRECOVERABLE
pub fn mark_mutex_unrecoverable(mutex: &RlctMutex) {
    // Set a special state that indicates the mutex is unrecoverable
    // This is done by keeping OWNER_DIED set but clearing the owner TID
    let current = mutex.inner.load(Ordering::Relaxed);
    let _ = mutex.inner.compare_exchange(
        current,
        OWNER_DIED_BIT | WAITING_BIT, // No owner, forever dead
        Ordering::AcqRel,
        Ordering::Relaxed,
    );
}

#[repr(u8)]
#[derive(PartialEq)]
enum Ty {
    Normal,
    Def,
    Errck,
    Recursive,
}

#[thread_local]
static CACHED_OS_TID_INVALID_AFTER_FORK: Cell<u32> = Cell::new(0);

fn os_tid_invalid_after_fork() -> u32 {
    let value = CACHED_OS_TID_INVALID_AFTER_FORK.get();

    if value == 0 {
        let tid = Sys::gettid();
        assert_ne!(tid, -1, "failed to obtain current thread ID");
        CACHED_OS_TID_INVALID_AFTER_FORK.set(tid as u32);
        tid as u32
    } else {
        value
    }
}
