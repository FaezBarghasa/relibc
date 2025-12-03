use core::{
    cell::UnsafeCell,
    fmt, ops,
    ptr::NonNull,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{header::time::timespec, pthread::Pshared};

pub struct InnerRwLock {
    state: AtomicU32,
}
// PTHREAD_RWLOCK_INITIALIZER is defined as "all zeroes".

const WRITER_BIT: u32 = 1 << (u32::BITS - 1);
const READER_MASK: u32 = !WRITER_BIT;

impl InnerRwLock {
    pub const fn new(_pshared: Pshared) -> Self {
        Self {
            state: AtomicU32::new(0),
        }
    }
    pub fn acquire_write_lock(&self, deadline: Option<&timespec>) {
        // Spin a few times, since this is usually faster than a syscall.
        for _ in 0..100 {
            if self.try_acquire_write_lock().is_ok() {
                return;
            }
            core::hint::spin_loop();
        }

        loop {
            let mut state = self.state.load(Ordering::Relaxed);
            if state & WRITER_BIT == 0 {
                match self.state.compare_exchange_weak(
                    state,
                    state | WRITER_BIT,
                    Ordering::Acquire,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(s) => {
                        state = s;
                        continue;
                    }
                }
            }
            let _ = crate::sync::futex_wait(&self.state, state, deadline);
        }

        loop {
            let state = self.state.load(Ordering::Relaxed);
            if state & READER_MASK == 0 {
                return;
            }
            let _ = crate::sync::futex_wait(&self.state, state, deadline);
        }
    }
    pub fn acquire_read_lock(&self, deadline: Option<&timespec>) {
        // Spin a few times, since this is usually faster than a syscall.
        for _ in 0..100 {
            if self.try_acquire_read_lock().is_ok() {
                return;
            }
            core::hint::spin_loop();
        }

        loop {
            let state = self.state.load(Ordering::Relaxed);

            if state & WRITER_BIT == 0 {
                if self
                    .state
                    .compare_exchange_weak(
                        state,
                        state + 1,
                        Ordering::Acquire,
                        Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    return;
                }
            }

            // Wait for the writer to finish.
            let _ = crate::sync::futex_wait(&self.state, state, deadline);
        }
    }
    pub fn try_acquire_read_lock(&self) -> Result<(), u32> {
        let mut state = self.state.load(Ordering::Acquire);

        loop {
            if state & WRITER_BIT != 0 {
                return Err(state);
            }

            match self.state.compare_exchange_weak(
                state,
                state + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(s) => state = s,
            }
        }
    }
    pub fn try_acquire_write_lock(&self) -> Result<(), u32> {
        match self.state.compare_exchange(
            0,
            WRITER_BIT,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(()),
            Err(s) => Err(s),
        }
    }

    pub fn unlock(&self) {
        let state = self.state.load(Ordering::Relaxed);
        if state & WRITER_BIT != 0 {
            self.state.fetch_and(!WRITER_BIT, Ordering::Release);
        } else {
            self.state.fetch_sub(1, Ordering::Release);
        }

        // Wake up waiting threads.
        let _ = crate::sync::futex_wake(&self.state, i32::MAX);
    }
}

pub struct RwLock<T: ?Sized> {
    inner: InnerRwLock,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(val: T) -> Self {
        Self {
            inner: InnerRwLock::new(Pshared::Private),
            data: UnsafeCell::new(val),
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    pub fn read(&self) -> ReadGuard<'_, T> {
        self.inner.acquire_read_lock(None);
        unsafe { ReadGuard::new(self) }
    }

    pub fn write(&self) -> WriteGuard<'_, T> {
        self.inner.acquire_write_lock(None);
        unsafe { WriteGuard::new(self) }
    }

    pub fn try_read(&self) -> Option<ReadGuard<'_, T>> {
        if self.inner.try_acquire_read_lock().is_ok() {
            Some(unsafe { ReadGuard::new(self) })
        } else {
            None
        }
    }

    pub fn try_write(&self) -> Option<WriteGuard<'_, T>> {
        if self.inner.try_acquire_write_lock().is_ok() {
            Some(unsafe { WriteGuard::new(self) })
        } else {
            None
        }
    }
}

pub struct ReadGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

impl<T: ?Sized> !Send for ReadGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for ReadGuard<'_, T> {}

impl<'a, T: ?Sized> ReadGuard<'a, T> {
    unsafe fn new(lock: &'a RwLock<T>) -> Self {
        Self { lock }
    }
}

impl<'a, T: ?Sized> ops::Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We have shared reference to the data.
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for ReadGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.inner.unlock();
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for ReadGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for ReadGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

pub struct WriteGuard<'a, T: ?Sized + 'a> {
    lock: &'a RwLock<T>,
}

impl<T: ?Sized> !Send for WriteGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for WriteGuard<'_, T> {}

impl<'a, T: ?Sized> WriteGuard<'a, T> {
    unsafe fn new(lock: &'a RwLock<T>) -> Self {
        Self { lock }
    }
}

impl<'a, T: ?Sized> ops::Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: We have exclusive reference to the data.
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> ops::DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: We have exclusive reference to the data.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.inner.unlock();
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for WriteGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for WriteGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
