#![forbid(unsafe_code)]

use crate::platform::types::*;
use crate::error::{Result, Errno};
use crate::header::errno::{EPERM, EINVAL, EBADF, ETIMEDOUT};
use crate::platform::{Pal, Sys};
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use crate::sync::Mutex as RelibcMutex;
use crate::sync::AtomicLock;
use core::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use alloc::vec::Vec;
use crate::header::time::timespec;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ntsync_sem_args {
    pub count: u32,
    pub max: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ntsync_mutex_args {
    pub owner: u32,
    pub count: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ntsync_event_args {
    pub manual: u32,
    pub signaled: u32,
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ntsync_wait_args {
    pub timeout: u64,
    pub objs: u64,
    pub count: u32,
    pub index: u32,
    pub flags: u32,
    pub owner: u32,
    pub alert: u32,
    pub pad: u32,
}

pub enum NtObject {
    Device,
    Mutex(Arc<NtMutex>),
    Semaphore(Arc<NtSemaphore>),
    Event(Arc<NtEvent>),
}

pub struct NtMutex {
    pub owner: AtomicU32,
    pub count: AtomicU32,
    pub waiters: RelibcMutex<Vec<Waiter>>,
}

pub struct NtSemaphore {
    pub count: AtomicU32,
    pub max: u32,
    pub waiters: RelibcMutex<Vec<Waiter>>,
}

pub struct NtEvent {
    pub manual: bool,
    pub signaled: AtomicU32,
    pub waiters: RelibcMutex<Vec<Waiter>>,
}

pub struct ThreadWait {
    pub lock: AtomicLock,
    pub signaled_index: AtomicU32,
}

#[derive(Clone)]
pub struct Waiter {
    pub thread_wait: Arc<ThreadWait>,
    pub index: u32,
    pub tid: u32,
    pub wait_all_shared: Option<Arc<WaitAllShared>>,
}

pub struct WaitAllShared {
    pub thread_wait: Arc<ThreadWait>,
    pub objs: Vec<Arc<NtObject>>,
    pub indices: Vec<u32>,
    pub acquired: AtomicBool,
}

static NTSYNC_REGISTRY: RelibcMutex<BTreeMap<c_int, Arc<NtObject>>> = RelibcMutex::new(BTreeMap::new());

fn register_object(fd: c_int, obj: NtObject) {
    NTSYNC_REGISTRY.lock().insert(fd, Arc::new(obj));
}

pub fn ntsync_close(fd: c_int) -> bool {
    NTSYNC_REGISTRY.lock().remove(&fd).is_some()
}

pub fn is_ntsync_fd(fd: c_int) -> bool {
    NTSYNC_REGISTRY.lock().contains_key(&fd)
}

fn get_object(fd: c_int) -> Option<Arc<NtObject>> {
    NTSYNC_REGISTRY.lock().get(&fd).cloned()
}

impl NtMutex {
    pub fn new(owner: u32, count: u32) -> Self {
        Self {
            owner: AtomicU32::new(owner),
            count: AtomicU32::new(count),
            waiters: RelibcMutex::new(Vec::new()),
        }
    }

    pub fn unlock(&self, owner: u32) -> Result<(u32, u32)> {
        let current_owner = self.owner.load(Ordering::SeqCst);
        if current_owner != owner {
            return Err(Errno(EPERM));
        }
        let current_count = self.count.load(Ordering::SeqCst);
        if current_count == 0 {
            return Err(Errno(EINVAL));
        }

        if current_count > 1 {
            self.count.store(current_count - 1, Ordering::SeqCst);
            return Ok((owner, current_count));
        }

        self.owner.store(0, Ordering::SeqCst);
        self.count.store(0, Ordering::SeqCst);
        self.wake_waiters();
        Ok((owner, current_count))
    }

    fn wake_waiters(&self) {
        let mut waiters = self.waiters.lock();
        let mut i = 0;
        while i < waiters.len() {
            let owner = self.owner.load(Ordering::SeqCst);
            if owner != 0 {
                break;
            }
            let waiter = &waiters[i];
            if let Some(ref wait_all) = waiter.wait_all_shared {
                if try_satisfy_wait_all(wait_all) {
                    waiters.remove(i);
                    continue;
                }
            } else {
                match self.owner.compare_exchange(0, waiter.tid, Ordering::SeqCst, Ordering::SeqCst) {
                    Ok(_) => {
                        self.count.store(1, Ordering::SeqCst);
                        waiter.thread_wait.signaled_index.store(waiter.index, Ordering::SeqCst);
                        crate::sync::futex_wake(&waiter.thread_wait.lock.atomic, 1);
                        waiters.remove(i);
                        continue;
                    }
                    Err(_) => {}
                }
            }
            i += 1;
        }
    }
}

impl NtSemaphore {
    pub fn new(count: u32, max: u32) -> Self {
        Self {
            count: AtomicU32::new(count),
            max,
            waiters: RelibcMutex::new(Vec::new()),
        }
    }

    pub fn post(&self, release_count: u32) -> u32 {
        if release_count == 0 {
            return self.count.load(Ordering::SeqCst);
        }
        let mut prev = self.count.load(Ordering::SeqCst);
        loop {
            let next = prev.checked_add(release_count).unwrap_or(prev);
            let next = if next > self.max { self.max } else { next };
            match self.count.compare_exchange_weak(prev, next, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => {
                    self.wake_waiters();
                    return prev;
                }
                Err(actual) => prev = actual,
            }
        }
    }

    fn wake_waiters(&self) {
        let mut waiters = self.waiters.lock();
        let mut i = 0;
        while i < waiters.len() {
            let count = self.count.load(Ordering::SeqCst);
            if count == 0 {
                break;
            }
            let waiter = &waiters[i];
            if let Some(ref wait_all) = waiter.wait_all_shared {
                if try_satisfy_wait_all(wait_all) {
                    waiters.remove(i);
                    continue;
                }
            } else {
                let mut prev = count;
                let mut satisfied = false;
                while prev > 0 {
                    match self.count.compare_exchange_weak(prev, prev - 1, Ordering::SeqCst, Ordering::SeqCst) {
                        Ok(_) => {
                            waiter.thread_wait.signaled_index.store(waiter.index, Ordering::SeqCst);
                            crate::sync::futex_wake(&waiter.thread_wait.lock.atomic, 1);
                            satisfied = true;
                            break;
                        }
                        Err(actual) => prev = actual,
                    }
                }
                if satisfied {
                    waiters.remove(i);
                    continue;
                }
            }
            i += 1;
        }
    }
}

impl NtEvent {
    pub fn new(manual: bool, signaled: bool) -> Self {
        Self {
            manual,
            signaled: AtomicU32::new(if signaled { 1 } else { 0 }),
            waiters: RelibcMutex::new(Vec::new()),
        }
    }

    pub fn set(&self) -> u32 {
        let prev = self.signaled.swap(1, Ordering::SeqCst);
        self.wake_waiters();
        prev
    }

    pub fn reset(&self) -> u32 {
        self.signaled.swap(0, Ordering::SeqCst)
    }

    pub fn pulse(&self) -> u32 {
        let prev = self.signaled.swap(1, Ordering::SeqCst);
        self.wake_waiters();
        self.signaled.store(0, Ordering::SeqCst);
        prev
    }

    fn wake_waiters(&self) {
        let mut waiters = self.waiters.lock();
        let mut i = 0;
        while i < waiters.len() {
            let signaled = self.signaled.load(Ordering::SeqCst);
            if signaled == 0 {
                break;
            }
            let waiter = &waiters[i];
            if let Some(ref wait_all) = waiter.wait_all_shared {
                if try_satisfy_wait_all(wait_all) {
                    if !self.manual {
                        self.signaled.store(0, Ordering::SeqCst);
                    }
                    waiters.remove(i);
                    continue;
                }
            } else {
                waiter.thread_wait.signaled_index.store(waiter.index, Ordering::SeqCst);
                crate::sync::futex_wake(&waiter.thread_wait.lock.atomic, 1);
                if !self.manual {
                    self.signaled.store(0, Ordering::SeqCst);
                }
                waiters.remove(i);
                continue;
            }
            i += 1;
        }
    }
}

fn unregister_waiter(thread_wait: &Arc<ThreadWait>, objs: &[Arc<NtObject>]) {
    for obj in objs {
        match **obj {
            NtObject::Mutex(ref m) => {
                m.waiters.lock().retain(|w| !Arc::ptr_eq(&w.thread_wait, thread_wait));
            }
            NtObject::Semaphore(ref s) => {
                s.waiters.lock().retain(|w| !Arc::ptr_eq(&w.thread_wait, thread_wait));
            }
            NtObject::Event(ref e) => {
                e.waiters.lock().retain(|w| !Arc::ptr_eq(&w.thread_wait, thread_wait));
            }
            _ => {}
        }
    }
}

fn lock_and_check(
    unique_objs: &[Arc<NtObject>],
    idx: usize,
    wait_all: &WaitAllShared,
    tid: u32,
) -> bool {
    if idx == unique_objs.len() {
        if wait_all.acquired.load(Ordering::SeqCst) {
            return true;
        }

        for obj in unique_objs {
            let occurrences = wait_all.objs.iter().filter(|o| Arc::ptr_eq(o, obj)).count() as u32;
            match **obj {
                NtObject::Mutex(ref m) => {
                    let owner = m.owner.load(Ordering::SeqCst);
                    if owner != 0 && owner != tid {
                        return false;
                    }
                }
                NtObject::Semaphore(ref s) => {
                    if s.count.load(Ordering::SeqCst) < occurrences {
                        return false;
                    }
                }
                NtObject::Event(ref e) => {
                    if e.signaled.load(Ordering::SeqCst) == 0 {
                        return false;
                    }
                }
                _ => return false,
            }
        }

        for obj in unique_objs {
            let occurrences = wait_all.objs.iter().filter(|o| Arc::ptr_eq(o, obj)).count() as u32;
            match **obj {
                NtObject::Mutex(ref m) => {
                    let owner = m.owner.load(Ordering::SeqCst);
                    if owner == 0 {
                        m.owner.store(tid, Ordering::SeqCst);
                        m.count.store(occurrences, Ordering::SeqCst);
                    } else {
                        m.count.store(m.count.load(Ordering::SeqCst) + occurrences, Ordering::SeqCst);
                    }
                }
                NtObject::Semaphore(ref s) => {
                    s.count.store(s.count.load(Ordering::SeqCst) - occurrences, Ordering::SeqCst);
                }
                NtObject::Event(ref e) => {
                    if !e.manual {
                        e.signaled.store(0, Ordering::SeqCst);
                    }
                }
                _ => {}
            }
        }

        wait_all.acquired.store(true, Ordering::SeqCst);
        wait_all.thread_wait.signaled_index.store(0, Ordering::SeqCst);
        crate::sync::futex_wake(&wait_all.thread_wait.lock.atomic, 1);
        return true;
    }

    let obj = &unique_objs[idx];
    match **obj {
        NtObject::Mutex(ref m) => {
            let _guard = m.waiters.lock();
            lock_and_check(unique_objs, idx + 1, wait_all, tid)
        }
        NtObject::Semaphore(ref s) => {
            let _guard = s.waiters.lock();
            lock_and_check(unique_objs, idx + 1, wait_all, tid)
        }
        NtObject::Event(ref e) => {
            let _guard = e.waiters.lock();
            lock_and_check(unique_objs, idx + 1, wait_all, tid)
        }
        _ => lock_and_check(unique_objs, idx + 1, wait_all, tid),
    }
}

fn try_satisfy_wait_all(wait_all: &WaitAllShared) -> bool {
    if wait_all.acquired.load(Ordering::SeqCst) {
        return true;
    }

    let mut unique_objs = wait_all.objs.clone();
    unique_objs.sort_by_key(|obj| Arc::as_ptr(obj) as usize);
    unique_objs.dedup_by(|a, b| Arc::ptr_eq(a, b));

    let tid = Sys::gettid() as u32;
    lock_and_check(&unique_objs, 0, wait_all, tid)
}

pub fn ntsync_open() -> Result<c_int> {
    let fd = Sys::open(c"/scheme/null".into(), crate::header::fcntl::O_RDONLY, 0)?;
    register_object(fd, NtObject::Device);
    Ok(fd)
}

pub fn ntsync_create_sem(count: u32, max: u32) -> Result<c_int> {
    let sem = Arc::new(NtSemaphore::new(count, max));
    let new_fd = Sys::open(c"/scheme/null".into(), crate::header::fcntl::O_RDONLY, 0)?;
    register_object(new_fd, NtObject::Semaphore(sem));
    Ok(new_fd)
}

pub fn ntsync_create_mutex(owner: u32, count: u32) -> Result<c_int> {
    let mutex = Arc::new(NtMutex::new(owner, count));
    let new_fd = Sys::open(c"/scheme/null".into(), crate::header::fcntl::O_RDONLY, 0)?;
    register_object(new_fd, NtObject::Mutex(mutex));
    Ok(new_fd)
}

pub fn ntsync_create_event(manual: bool, signaled: bool) -> Result<c_int> {
    let event = Arc::new(NtEvent::new(manual, signaled));
    let new_fd = Sys::open(c"/scheme/null".into(), crate::header::fcntl::O_RDONLY, 0)?;
    register_object(new_fd, NtObject::Event(event));
    Ok(new_fd)
}

pub fn ntsync_sem_post(fd: c_int, release_count: u32) -> Result<u32> {
    let obj = get_object(fd).ok_or(Errno(EBADF))?;
    if let NtObject::Semaphore(ref sem) = *obj {
        Ok(sem.post(release_count))
    } else {
        Err(Errno(EINVAL))
    }
}

pub fn ntsync_mutex_unlock(fd: c_int, owner: u32) -> Result<(u32, u32)> {
    let obj = get_object(fd).ok_or(Errno(EBADF))?;
    if let NtObject::Mutex(ref mutex) = *obj {
        mutex.unlock(owner)
    } else {
        Err(Errno(EINVAL))
    }
}

pub fn ntsync_event_set(fd: c_int) -> Result<u32> {
    let obj = get_object(fd).ok_or(Errno(EBADF))?;
    if let NtObject::Event(ref event) = *obj {
        Ok(event.set())
    } else {
        Err(Errno(EINVAL))
    }
}

pub fn ntsync_event_reset(fd: c_int) -> Result<u32> {
    let obj = get_object(fd).ok_or(Errno(EBADF))?;
    if let NtObject::Event(ref event) = *obj {
        Ok(event.reset())
    } else {
        Err(Errno(EINVAL))
    }
}

pub fn ntsync_event_pulse(fd: c_int) -> Result<u32> {
    let obj = get_object(fd).ok_or(Errno(EBADF))?;
    if let NtObject::Event(ref event) = *obj {
        Ok(event.pulse())
    } else {
        Err(Errno(EINVAL))
    }
}

pub fn ntsync_sem_read(fd: c_int) -> Result<(u32, u32)> {
    let obj = get_object(fd).ok_or(Errno(EBADF))?;
    if let NtObject::Semaphore(ref sem) = *obj {
        Ok((sem.count.load(Ordering::SeqCst), sem.max))
    } else {
        Err(Errno(EINVAL))
    }
}

pub fn ntsync_mutex_read(fd: c_int) -> Result<(u32, u32)> {
    let obj = get_object(fd).ok_or(Errno(EBADF))?;
    if let NtObject::Mutex(ref mutex) = *obj {
        Ok((mutex.owner.load(Ordering::SeqCst), mutex.count.load(Ordering::SeqCst)))
    } else {
        Err(Errno(EINVAL))
    }
}

pub fn ntsync_event_read(fd: c_int) -> Result<(bool, u32)> {
    let obj = get_object(fd).ok_or(Errno(EBADF))?;
    if let NtObject::Event(ref event) = *obj {
        Ok((event.manual, event.signaled.load(Ordering::SeqCst)))
    } else {
        Err(Errno(EINVAL))
    }
}

pub fn ntsync_wait_any(handles: &[c_int], timeout: u64) -> Result<u32> {
    let tid = Sys::gettid() as u32;

    let mut objs = Vec::new();
    for &h in handles {
        let obj = get_object(h).ok_or(Errno(EBADF))?;
        objs.push(obj);
    }

    let thread_wait = Arc::new(ThreadWait {
        lock: AtomicLock::new(0),
        signaled_index: AtomicU32::new(u32::MAX),
    });

    for (i, obj) in objs.iter().enumerate() {
        match **obj {
            NtObject::Mutex(ref m) => {
                let _guard = m.waiters.lock();
                let owner = m.owner.load(Ordering::SeqCst);
                if owner == 0 {
                    m.owner.store(tid, Ordering::SeqCst);
                    m.count.store(1, Ordering::SeqCst);
                    return Ok(i as u32);
                } else if owner == tid {
                    m.count.store(m.count.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
                    return Ok(i as u32);
                }
            }
            NtObject::Semaphore(ref s) => {
                let _guard = s.waiters.lock();
                let count = s.count.load(Ordering::SeqCst);
                if count > 0 {
                    s.count.store(count - 1, Ordering::SeqCst);
                    return Ok(i as u32);
                }
            }
            NtObject::Event(ref e) => {
                let _guard = e.waiters.lock();
                let signaled = e.signaled.load(Ordering::SeqCst);
                if signaled > 0 {
                    if !e.manual {
                        e.signaled.store(0, Ordering::SeqCst);
                    }
                    return Ok(i as u32);
                }
            }
            _ => return Err(Errno(EINVAL)),
        }
    }

    if timeout == 0 {
        return Err(Errno(ETIMEDOUT));
    }

    for (i, obj) in objs.iter().enumerate() {
        let waiter = Waiter {
            thread_wait: thread_wait.clone(),
            index: i as u32,
            tid,
            wait_all_shared: None,
        };
        match **obj {
            NtObject::Mutex(ref m) => m.waiters.lock().push(waiter),
            NtObject::Semaphore(ref s) => s.waiters.lock().push(waiter),
            NtObject::Event(ref e) => e.waiters.lock().push(waiter),
            _ => {}
        }
    }

    let deadline = if timeout != u64::MAX {
        let ts = timespec {
            tv_sec: (timeout / 1_000_000_000) as time_t,
            tv_nsec: (timeout % 1_000_000_000) as c_long,
        };
        Some(ts)
    } else {
        None
    };

    loop {
        let idx = thread_wait.signaled_index.load(Ordering::SeqCst);
        if idx != u32::MAX {
            return Ok(idx);
        }
        let res = crate::sync::futex_wait(&thread_wait.lock.atomic, 0, deadline.as_ref());
        if res == crate::sync::FutexWaitResult::TimedOut {
            unregister_waiter(&thread_wait, &objs);
            return Err(Errno(ETIMEDOUT));
        }
    }
}

pub fn ntsync_wait_all(handles: &[c_int], timeout: u64) -> Result<()> {
    let tid = Sys::gettid() as u32;

    let mut objs = Vec::new();
    for &h in handles {
        let obj = get_object(h).ok_or(Errno(EBADF))?;
        objs.push(obj);
    }

    let thread_wait = Arc::new(ThreadWait {
        lock: AtomicLock::new(0),
        signaled_index: AtomicU32::new(u32::MAX),
    });

    let wait_all_shared = Arc::new(WaitAllShared {
        thread_wait: thread_wait.clone(),
        objs: objs.clone(),
        indices: (0..handles.len() as u32).collect(),
        acquired: AtomicBool::new(false),
    });

    if try_satisfy_wait_all(&wait_all_shared) {
        return Ok(());
    }

    if timeout == 0 {
        return Err(Errno(ETIMEDOUT));
    }

    for (i, obj) in objs.iter().enumerate() {
        let waiter = Waiter {
            thread_wait: thread_wait.clone(),
            index: i as u32,
            tid,
            wait_all_shared: Some(wait_all_shared.clone()),
        };
        match **obj {
            NtObject::Mutex(ref m) => m.waiters.lock().push(waiter),
            NtObject::Semaphore(ref s) => s.waiters.lock().push(waiter),
            NtObject::Event(ref e) => e.waiters.lock().push(waiter),
            _ => {}
        }
    }

    let deadline = if timeout != u64::MAX {
        let ts = timespec {
            tv_sec: (timeout / 1_000_000_000) as time_t,
            tv_nsec: (timeout % 1_000_000_000) as c_long,
        };
        Some(ts)
    } else {
        None
    };

    loop {
        if wait_all_shared.acquired.load(Ordering::SeqCst) {
            return Ok(());
        }
        let res = crate::sync::futex_wait(&thread_wait.lock.atomic, 0, deadline.as_ref());
        if res == crate::sync::FutexWaitResult::TimedOut {
            unregister_waiter(&thread_wait, &objs);
            return Err(Errno(ETIMEDOUT));
        }
    }
}
