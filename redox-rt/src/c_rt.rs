//! C-specific runtime components.
//!
//! This file is not part of `redox-rt` proper, but is instead intended to be used by `relibc`'s
//! `c-rt-pre` crate.

use crate::Tcb;

// TODO: Should we use a different mechanism for this?
#[no_mangle]
static mut __tcb_impl: Tcb = Tcb {
    tcb_ptr: core::ptr::null_mut(),
    tcb_len: 0,
    tls_end: core::ptr::null_mut(),
    dtv: core::ptr::null_mut(),
    dtv_len: 0,
    os_specific: crate::RtTcb {
        control: syscall::Sigcontrol::new(),
        arch: core::cell::UnsafeCell::new(crate::arch::SigArea::default()),
        thr_fd: core::cell::UnsafeCell::new(None),
    },
    platform_specific: crate::arch::TcbExtension {
        self_ptr: core::ptr::null_mut(),
        stack_base: core::ptr::null_mut(),
        stack_size: 0,
        tls_dtv: core::ptr::null_mut(),
        tls_dtv_len: 0,
        tls_static_base: core::ptr::null_mut(),
    },
};

#[cfg(not(feature = "expect-tls-free"))]
compile_error!("The c-rt feature expects the expect-tls-free feature to be enabled.");

pub trait ExpectTlsFree {
    fn expect_notls(self, msg: &str);
}
impl<T, E> ExpectTlsFree for Result<T, E> {
    fn expect_notls(self, msg: &str) {
        if self.is_err() {
            // TODO: Use a proper printing mechanism.
            let _ = syscall::write(2, msg.as_bytes());
            let _ = syscall::write(2, b"\n");
            syscall::exit(1);
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
