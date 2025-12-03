use core::{
    num::NonZeroU32,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    header::pthread::{RlctCond, RlctMutex},
    sync::Mutex,
};

pub struct Barrier {
    original_count: NonZeroU32,
    count: AtomicU32,
    gen_id: AtomicU32,
}

pub enum WaitResult {
    Waited,
    NotifiedAll,
}

impl Barrier {
    pub fn new(count: NonZeroU32) -> Self {
        Self {
            original_count: count,
            count: AtomicU32::new(0),
            gen_id: AtomicU32::new(0),
        }
    }
    pub fn wait(&self) -> WaitResult {
        let old_gen_id = self.gen_id.load(Ordering::Relaxed);
        let count = self.count.fetch_add(1, Ordering::AcqRel);

        if count == self.original_count.get() - 1 {
            self.count.store(0, Ordering::Relaxed);
            self.gen_id.fetch_add(1, Ordering::Relaxed);
            crate::sync::futex_wake(&self.gen_id, i32::MAX);
            WaitResult::NotifiedAll
        } else {
            while self.gen_id.load(Ordering::Relaxed) == old_gen_id {
                crate::sync::futex_wait(&self.gen_id, old_gen_id, None);
            }
            WaitResult::Waited
        }
    }
}
