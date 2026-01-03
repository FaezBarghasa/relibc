//! C-specific runtime components.
//!
//! This file is not part of `redox-rt` proper, but is instead intended to be used by `relibc`'s
//! `c-rt-pre` crate.

use crate::Tcb;
use syscall::{flag::*, number::*};

// TODO: Should we use a different mechanism for this?
#[no_mangle]
static mut __tcb_impl: Tcb = Tcb {
    tcb_ptr: core::ptr::null_mut(),
    tcb_len: 0,
    tls_end: core::ptr::null_mut(),
    dtv: core::ptr::null_mut(),
    dtv_len: 0,
    os_specific: crate::RtTcb {
        control: unsafe { core::mem::zeroed() },
        arch: core::cell::UnsafeCell::new(unsafe { core::mem::zeroed() }),
        thr_fd: core::cell::UnsafeCell::new(None),
    },
    platform_specific: crate::arch::TcbExtension {
        self_ptr: core::ptr::null_mut(),
        stack_base: core::ptr::null_mut(),
        stack_size: 0,
        tls_dtv: core::ptr::null_mut(),
        tls_dtv_len: 0,
        tls_static_base: core::ptr::null_mut(),
        my_thread_local: 0,
    },
};

#[cfg(not(feature = "expect-tls-free"))]
compile_error!("The c-rt feature expects the expect-tls-free feature to be enabled.");

pub trait ExpectTlsFree<T> {
    fn expect_notls(self, msg: &str) -> T;
}
impl<T, E> ExpectTlsFree<T> for Result<T, E> {
    fn expect_notls(self, msg: &str) -> T {
        match self {
            Ok(v) => v,
            Err(_) => {
                // TODO: Use a proper printing mechanism.
                let _ = syscall::write(2, msg.as_bytes());
                let _ = syscall::write(2, b"\n");
                let _ = unsafe { syscall::syscall1(1, 1) };
                unreachable!()
            }
        }
    }
}
#[macro_export]
macro_rules! c_str {
    ($str:literal) => {
        concat!($str, "\0").as_ptr()
    };
}
#[macro_export]
macro_rules! c {
    ($str:literal) => {
        concat!($str, "\0")
    };
}
