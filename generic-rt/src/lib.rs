#![no_std]
#![allow(internal_features)]
#![feature(core_intrinsics)]

use core::{
    arch::asm,
    mem::{self, offset_of},
};

#[derive(Debug)]
#[repr(C)]
pub struct GenericTcb<Os, Platform> {
    /// Pointer to this structure
    pub tcb_ptr: *mut Self,
    /// Size of the memory allocated for this structure in bytes (should be same as page size)
    pub tcb_len: usize,
    /// Pointer to the end of static TLS.
    pub tls_end: *mut u8,
    pub dtv: *mut (),
    pub dtv_len: usize,
    pub os_specific: Os,
    pub platform_specific: Platform,
}
impl<Os, Platform> GenericTcb<Os, Platform> {
    /// Architecture specific code to read a usize from the TCB - aarch64
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline(always)]
    #[cfg(target_arch = "aarch64")]
    pub unsafe fn arch_read_self() -> *mut Self {
        let tcb_ptr: *mut Self;
        asm!("mrs {}, tpidr_el0", out(reg) tcb_ptr);
        tcb_ptr
    }

    /// Architecture specific code to read a usize from the TCB - x86
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline(always)]
    #[cfg(target_arch = "x86")]
    pub unsafe fn arch_read_self() -> *mut Self {
        let value;
        asm!(
            "
            mov {}, gs:[{}]
            ",
            out(reg) value,
            in(reg) offset_of!(Self, platform_specific.self_ptr),
        );
        value
    }

    /// Architecture specific code to read a usize from the TCB - x86_64
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline(always)]
    #[cfg(target_arch = "x86_64")]
    pub unsafe fn arch_read_self() -> *mut Self {
        let value;
        asm!(
            "
            mov {}, fs:[0]
            ",
            out(reg) value,
        );
        value
    }

    /// Architecture specific code to read a usize from the TCB - riscv64
    #[allow(unsafe_op_in_unsafe_fn)]
    #[inline(always)]
    #[cfg(target_arch = "riscv64")]
    unsafe fn arch_read_self() -> *mut Self {
        let value;
        asm!(
            "mv {value}, tp",
            value = out(reg) value,
        );
        value
    }

    pub unsafe fn current_ptr() -> Option<*mut Self> {
        let tcb_ptr = unsafe { Self::arch_read_self() };
        if tcb_ptr.is_null() || (*tcb_ptr).tcb_len < mem::size_of::<Self>() {
            None
        } else {
            Some(tcb_ptr)
        }
    }
    pub unsafe fn current() -> Option<&'static mut Self> {
        unsafe { Some(&mut *Self::current_ptr()?) }
    }
}
pub fn panic_notls(_msg: impl core::fmt::Display) -> ! {
    // TODO: actually print _msg, perhaps by having panic_notls take a `T: DebugBackend` that can
    // propagate until called by e.g. relibc start
    core::intrinsics::abort();
}

#[cfg(feature = "expect-tls-free")]
pub trait ExpectTlsFree {
    type Unwrapped;

    fn expect_notls(self, msg: &str) -> Self::Unwrapped;
}
#[cfg(feature = "expect-tls-free")]
impl<T, E: core::fmt::Debug> ExpectTlsFree for Result<T, E> {
    type Unwrapped = T;

    fn expect_notls(self, msg: &str) -> T {
        match self {
            Ok(t) => t,
            Err(err) => panic_notls(format_args!(
                "{msg}: expect failed for Result with err: {err:?}",
            )),
        }
    }
}
#[cfg(feature = "expect-tls-free")]
impl<T> ExpectTlsFree for Option<T> {
    type Unwrapped = T;

    fn expect_notls(self, msg: &str) -> T {
        match self {
            Some(t) => t,
            None => panic_notls(format_args!("{msg}: expect failed for Option")),
        }
    }
}
