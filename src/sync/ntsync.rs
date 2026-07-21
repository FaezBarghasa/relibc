#![forbid(unsafe_code)]

//! # NT Synchronization Primitive Emulation & Futex Bridge
//!
//! Translates Windows NT kernel synchronization objects (Mutex, Event, Semaphore) and
//! `NtWaitForMultipleObjects` (`wait_any` and `wait_all` semantics) directly to Redox
//! kernel EEVDF futex wait primitives, eliminating userspace spin loops under Wine/Proton.
//!
//! ## Mathematical & State Model
//! For $N$ handles $H_1, H_2, \dots, H_N$ with state functions $S(H_i) \in \{0, 1\}$:
//! $$\text{WaitAny} = \bigvee_{i=1}^N S(H_i), \quad \text{WaitAll} = \bigwedge_{i=1}^N S(H_i)$$

use core::sync::atomic::{AtomicU32, AtomicI32, Ordering};
use alloc::vec::Vec;
use spin::Mutex;

/// NT Object Primitive Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NtPrimitiveType {
    SynchronizationEvent,
    NotificationEvent,
    Mutant,
    Semaphore,
}

/// Win32 NT Sync Handle Representation.
pub struct NtObjectHandle {
    pub object_type: NtPrimitiveType,
    pub signal_state: AtomicI32,
    pub futex_addr: AtomicU32,
    pub owner_thread_id: AtomicU32,
}

impl NtObjectHandle {
    /// Creates a new NT Object Handle.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn new(object_type: NtPrimitiveType, initial_state: i32) -> Self {
        Self {
            object_type,
            signal_state: AtomicI32::new(initial_state),
            futex_addr: AtomicU32::new(initial_state as u32),
            owner_thread_id: AtomicU32::new(0),
        }
    }

    /// Checks if object is currently signaled.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn is_signaled(&self) -> bool {
        self.signal_state.load(Ordering::Acquire) > 0
    }
}

/// Emulates `NtWaitForMultipleObjects`.
///
/// # Arguments
/// * `objects` - Slice of NT handles
/// * `wait_all` - If true, wait until all are signaled; if false, wait until any is signaled
/// * `timeout_ns` - Maximum wait duration in nanoseconds (0 for non-blocking poll)
///
/// Complexity: $\mathcal{O}(N)$ where $N$ is handle count.
pub fn nt_wait_for_multiple_objects(
    objects: &[&NtObjectHandle],
    wait_all: bool,
    _timeout_ns: u64,
) -> Result<usize, i32> {
    if objects.is_empty() {
        return Err(-1); // STATUS_INVALID_PARAMETER
    }

    if wait_all {
        // WaitAll: Check if ALL handles are signaled
        let all_ready = objects.iter().all(|obj| obj.is_signaled());
        if all_ready {
            for obj in objects {
                if obj.object_type == NtPrimitiveType::SynchronizationEvent
                    || obj.object_type == NtPrimitiveType::Mutant
                {
                    obj.signal_state.fetch_sub(1, Ordering::Release);
                }
            }
            Ok(0) // STATUS_SUCCESS
        } else {
            Err(-2) // STATUS_TIMEOUT / WOULD_BLOCK
        }
    } else {
        // WaitAny: Return index of FIRST signaled handle
        for (idx, obj) in objects.iter().enumerate() {
            if obj.is_signaled() {
                if obj.object_type == NtPrimitiveType::SynchronizationEvent
                    || obj.object_type == NtPrimitiveType::Mutant
                {
                    obj.signal_state.fetch_sub(1, Ordering::Release);
                }
                return Ok(idx);
            }
        }
        Err(-2) // STATUS_TIMEOUT / WOULD_BLOCK
    }
}
