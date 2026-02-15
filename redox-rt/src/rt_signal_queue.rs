//! Real-Time Signal Queue Implementation
//!
//! Provides FIFO-ordered queuing for real-time signals (SIGRTMIN to SIGRTMAX).
//! Standard signals (1-31) use the existing bitmap-based pending mechanism.
//! Real-time signals (32-63) are queued with their siginfo data preserved.
//!
//! ## POSIX Requirements
//! - RT signals delivered in FIFO order
//! - Each signal can be queued up to SIGQUEUE_MAX times
//! - siginfo_t must be preserved including si_value

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Maximum number of pending signals per signal number
pub const SIGQUEUE_MAX: usize = 32;

/// Total RT signals (SIGRTMIN=32 to SIGRTMAX=63)
pub const RT_SIGNAL_COUNT: usize = 32;

/// First real-time signal number
pub const SIGRTMIN: usize = 32;

/// Last real-time signal number
pub const SIGRTMAX: usize = 63;

/// Queued signal entry containing siginfo data
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QueuedSignal {
    /// Signal number (32-63)
    pub signo: u32,
    /// Signal code (SI_USER, SI_QUEUE, etc.)
    pub code: i32,
    /// Sending process ID
    pub pid: u32,
    /// Sending user ID
    pub uid: u32,
    /// User-provided value (from sigqueue)
    pub value: usize,
}

impl Default for QueuedSignal {
    fn default() -> Self {
        Self {
            signo: 0,
            code: 0,
            pid: 0,
            uid: 0,
            value: 0,
        }
    }
}

/// Per-signal queue using a circular buffer
pub struct SignalQueue {
    /// Circular buffer of queued signals
    buffer: [QueuedSignal; SIGQUEUE_MAX],
    /// Read index (consumer)
    head: AtomicUsize,
    /// Write index (producer)
    tail: AtomicUsize,
    /// Number of dropped signals due to queue full
    dropped: AtomicU32,
}

impl SignalQueue {
    pub const fn new() -> Self {
        Self {
            buffer: [QueuedSignal {
                signo: 0,
                code: 0,
                pid: 0,
                uid: 0,
                value: 0,
            }; SIGQUEUE_MAX],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            dropped: AtomicU32::new(0),
        }
    }

    /// Check if queue is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }

    /// Get number of queued signals
    #[inline]
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    /// Enqueue a signal (returns false if queue is full)
    pub fn enqueue(&mut self, signal: QueuedSignal) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);

        // Check if full
        if tail.wrapping_sub(head) >= SIGQUEUE_MAX {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return false;
        }

        // Write signal to buffer
        let idx = tail % SIGQUEUE_MAX;
        self.buffer[idx] = signal;

        // Advance tail with release ordering
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        true
    }

    /// Dequeue a signal (FIFO order)
    pub fn dequeue(&mut self) -> Option<QueuedSignal> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let idx = head % SIGQUEUE_MAX;
        let signal = self.buffer[idx];

        // Advance head with release ordering
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(signal)
    }

    /// Peek at the next signal without removing it
    pub fn peek(&self) -> Option<QueuedSignal> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let idx = head % SIGQUEUE_MAX;
        Some(self.buffer[idx])
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.head.store(0, Ordering::Relaxed);
        self.tail.store(0, Ordering::Relaxed);
    }

    /// Get and reset dropped signal count
    pub fn take_dropped(&self) -> u32 {
        self.dropped.swap(0, Ordering::Relaxed)
    }
}

/// Process-wide RT signal queue state
pub struct RtSignalQueues {
    /// One queue per RT signal (SIGRTMIN to SIGRTMAX)
    queues: [SignalQueue; RT_SIGNAL_COUNT],
    /// Bitmap of signals with pending queue entries
    pending_mask: AtomicU32,
}

impl RtSignalQueues {
    pub const fn new() -> Self {
        Self {
            queues: [const { SignalQueue::new() }; RT_SIGNAL_COUNT],
            pending_mask: AtomicU32::new(0),
        }
    }

    /// Map signal number to queue index
    #[inline]
    fn sig_to_idx(signo: u32) -> Option<usize> {
        let sig = signo as usize;
        if sig >= SIGRTMIN && sig <= SIGRTMAX {
            Some(sig - SIGRTMIN)
        } else {
            None
        }
    }

    /// Queue a real-time signal
    pub fn queue_signal(&mut self, signal: QueuedSignal) -> bool {
        let idx = match Self::sig_to_idx(signal.signo) {
            Some(i) => i,
            None => return false,
        };

        if self.queues[idx].enqueue(signal) {
            // Set pending bit
            self.pending_mask.fetch_or(1u32 << idx, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// Dequeue the next pending RT signal (lowest signal number first)
    pub fn dequeue_next(&mut self) -> Option<QueuedSignal> {
        let mask = self.pending_mask.load(Ordering::Acquire);
        if mask == 0 {
            return None;
        }

        // Find lowest pending signal (POSIX: deliver in signal number order)
        let idx = mask.trailing_zeros() as usize;
        if idx >= RT_SIGNAL_COUNT {
            return None;
        }

        let signal = self.queues[idx].dequeue();

        // Clear pending bit if queue is now empty
        if self.queues[idx].is_empty() {
            self.pending_mask
                .fetch_and(!(1u32 << idx), Ordering::Release);
        }

        signal
    }

    /// Dequeue a specific signal
    pub fn dequeue_signal(&mut self, signo: u32) -> Option<QueuedSignal> {
        let idx = Self::sig_to_idx(signo)?;
        let signal = self.queues[idx].dequeue();

        if self.queues[idx].is_empty() {
            self.pending_mask
                .fetch_and(!(1u32 << idx), Ordering::Release);
        }

        signal
    }

    /// Check if a specific signal has queued entries
    pub fn has_pending(&self, signo: u32) -> bool {
        let idx = match Self::sig_to_idx(signo) {
            Some(i) => i,
            None => return false,
        };
        !self.queues[idx].is_empty()
    }

    /// Get pending RT signal mask
    pub fn pending_mask(&self) -> u32 {
        self.pending_mask.load(Ordering::Acquire)
    }

    /// Clear all pending signals for a specific signal number
    pub fn clear_signal(&mut self, signo: u32) {
        let idx = match Self::sig_to_idx(signo) {
            Some(i) => i,
            None => return,
        };
        self.queues[idx].clear();
        self.pending_mask
            .fetch_and(!(1u32 << idx), Ordering::Release);
    }

    /// Get total dropped count across all queues
    pub fn total_dropped(&self) -> u32 {
        self.queues.iter().map(|q| q.take_dropped()).sum()
    }
}

/// Global RT signal queues (per-process)
static mut RT_QUEUES: RtSignalQueues = RtSignalQueues::new();

/// Queue a real-time signal with siginfo data
///
/// # Safety
/// This function modifies global state and must be called with signals disabled
pub unsafe fn queue_rt_signal(signo: u32, code: i32, pid: u32, uid: u32, value: usize) -> bool {
    let signal = QueuedSignal {
        signo,
        code,
        pid,
        uid,
        value,
    };
    unsafe { RT_QUEUES.queue_signal(signal) }
}

/// Dequeue the next pending RT signal
///
/// # Safety
/// This function modifies global state and must be called with signals disabled
pub unsafe fn dequeue_rt_signal() -> Option<QueuedSignal> {
    unsafe { RT_QUEUES.dequeue_next() }
}

/// Dequeue a specific RT signal
///
/// # Safety
/// This function modifies global state and must be called with signals disabled
pub unsafe fn dequeue_specific_rt_signal(signo: u32) -> Option<QueuedSignal> {
    unsafe { RT_QUEUES.dequeue_signal(signo) }
}

/// Check if any RT signals are pending
pub fn has_pending_rt_signals() -> bool {
    unsafe { RT_QUEUES.pending_mask() != 0 }
}

/// Get the RT signal pending mask
pub fn get_rt_pending_mask() -> u32 {
    unsafe { RT_QUEUES.pending_mask() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_queue_basic() {
        let mut queue = SignalQueue::new();

        assert!(queue.is_empty());

        let sig = QueuedSignal {
            signo: 34,
            code: 1,
            pid: 100,
            uid: 1000,
            value: 42,
        };

        assert!(queue.enqueue(sig));
        assert!(!queue.is_empty());
        assert_eq!(queue.len(), 1);

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.signo, 34);
        assert_eq!(dequeued.value, 42);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_signal_queue_fifo() {
        let mut queue = SignalQueue::new();

        for i in 0..5 {
            let sig = QueuedSignal {
                signo: 34,
                code: 1,
                pid: 100,
                uid: 1000,
                value: i,
            };
            assert!(queue.enqueue(sig));
        }

        for i in 0..5 {
            let sig = queue.dequeue().unwrap();
            assert_eq!(sig.value, i);
        }
    }

    #[test]
    fn test_rt_queues_order() {
        let mut queues = RtSignalQueues::new();

        // Queue signals in reverse order
        for signo in (SIGRTMIN..=SIGRTMIN + 3).rev() {
            let sig = QueuedSignal {
                signo: signo as u32,
                code: 1,
                pid: 100,
                uid: 1000,
                value: signo,
            };
            assert!(queues.queue_signal(sig));
        }

        // Should dequeue in signal number order (lowest first)
        for signo in SIGRTMIN..=SIGRTMIN + 3 {
            let sig = queues.dequeue_next().unwrap();
            assert_eq!(sig.signo as usize, signo);
        }
    }
}
