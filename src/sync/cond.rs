// Used design from https://www.remlab.net/op/futex-condvar.shtml

use crate::{
    error::Errno,
    header::{errno::*, pthread::*, time::timespec},
};

use core::sync::atomic::{AtomicU32 as AtomicUint, Ordering};

pub struct Cond {
    cur: AtomicUint,
}

type Result<T, E = Errno> = core::result::Result<T, E>;

impl Cond {
    pub fn new() -> Self {
        Self {
            cur: AtomicUint::new(0),
        }
    }
    pub fn broadcast(&self) -> Result<(), Errno> {
        self.cur.fetch_add(1, Ordering::Relaxed);
        crate::sync::futex_wake(&self.cur, i32::MAX);
        Ok(())
    }
    pub fn signal(&self) -> Result<(), Errno> {
        self.cur.fetch_add(1, Ordering::Relaxed);
        crate::sync::futex_wake(&self.cur, 1);
        Ok(())
    }
    pub fn timedwait(&self, mutex: &RlctMutex, timeout: &timespec) -> Result<(), Errno> {
        self.wait_inner(mutex, Some(timeout))
    }
    fn wait_inner(&self, mutex: &RlctMutex, timeout: Option<&timespec>) -> Result<(), Errno> {
        self.wait_inner_generic(
            || mutex.unlock(),
            || mutex.lock(),
            |timeout| mutex.lock_with_timeout(timeout),
            timeout,
        )
    }
    pub fn wait_inner_typedmutex<'lock, T>(
        &self,
        guard: crate::sync::MutexGuard<'lock, T>,
    ) -> crate::sync::MutexGuard<'lock, T> {
        let mut newguard = None;
        let lock = guard.mutex;
        self.wait_inner_generic(
            move || {
                drop(guard);
                Ok(())
            },
            || {
                newguard = Some(lock.lock());
                Ok(())
            },
            |_| unreachable!(),
            None,
        )
        .unwrap();
        newguard.unwrap()
    }
    // TODO: FUTEX_REQUEUE
    fn wait_inner_generic(
        &self,
        unlock: impl FnOnce() -> Result<()>,
        lock: impl FnOnce() -> Result<()>,
        lock_with_timeout: impl FnOnce(&timespec) -> Result<()>,
        deadline: Option<&timespec>,
    ) -> Result<(), Errno> {
        // TODO: Error checking for certain types (i.e. robust and errorcheck) of mutexes, e.g. if the
        // mutex is not locked.
        let current = self.cur.load(Ordering::Relaxed);

        unlock()?;

        let res = crate::sync::futex_wait(&self.cur, current, deadline);

        // Always re-acquire the lock, even if the wait timed out.
        if let Some(deadline) = deadline {
            lock_with_timeout(deadline)?;
        } else {
            lock()?;
        }

        if res == super::FutexWaitResult::TimedOut {
            return Err(Errno(ETIMEDOUT));
        }

        Ok(())
    }
    pub fn wait(&self, mutex: &RlctMutex) -> Result<(), Errno> {
        self.wait_inner(mutex, None)
    }
}
