//! fenv implementation for Redox, following http://pubs.opengroup.org/onlinepubs/7908799/xsh/fenv.h.html

use crate::platform::types::*;
use core::arch::asm;

#[cfg(target_arch = "aarch64")]
#[repr(C)]
pub struct fenv_t {
    pub __fpcr: u32,
    pub __fpsr: u32,
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
pub struct fexcept_t {
    pub __except: u32,
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
pub struct fenv_t {
    pub __fcsr: u32,
}

#[cfg(target_arch = "riscv64")]
#[repr(C)]
pub struct fexcept_t {
    pub __except: u32,
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[repr(C)]
pub struct fenv_t {
    //TODO
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[repr(C)]
pub struct fexcept_t {
    //TODO
}

pub const FE_DIVBYZERO: c_int = 1;
pub const FE_INEXACT: c_int = 2;
pub const FE_INVALID: c_int = 4;
pub const FE_OVERFLOW: c_int = 8;
pub const FE_UNDERFLOW: c_int = 16;
pub const FE_ALL_EXCEPT: c_int = 31;

pub const FE_DOWNWARD: c_int = 0b01;
pub const FE_TONEAREST: c_int = 0b00;
pub const FE_TOWARDZERO: c_int = 0b10;
pub const FE_UPWARD: c_int = 0b11;

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
#[no_mangle]
pub unsafe extern "C" fn feclearexcept(excepts: c_int) -> c_int {
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    fcsr &= !(excepts as u32);
    asm!("fscsr {}", in(reg) fcsr);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn feclearexcept(excepts: c_int) -> c_int {
    unimplemented!();
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
#[no_mangle]
pub unsafe extern "C" fn fegetexceptflag(flagp: *mut fexcept_t, excepts: c_int) -> c_int {
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    (*flagp).__except = fcsr & (excepts as u32);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn fegetexceptflag(flagp: *mut fexcept_t, excepts: c_int) -> c_int {
    unimplemented!();
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
#[no_mangle]
pub unsafe extern "C" fn feraiseexcept(excepts: c_int) -> c_int {
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    fcsr |= excepts as u32;
    asm!("fscsr {}", in(reg) fcsr);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn feraiseexcept(excepts: c_int) -> c_int {
    unimplemented!();
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
#[no_mangle]
pub unsafe extern "C" fn fesetexceptflag(flagp: *const fexcept_t, excepts: c_int) -> c_int {
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    fcsr &= !(excepts as u32);
    fcsr |= (*flagp).__except & (excepts as u32);
    asm!("fscsr {}", in(reg) fcsr);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn fesetexceptflag(flagp: *const fexcept_t, excepts: c_int) -> c_int {
    unimplemented!();
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
#[no_mangle]
pub unsafe extern "C" fn fetestexcept(excepts: c_int) -> c_int {
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    (fcsr as c_int) & excepts
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn fetestexcept(excepts: c_int) -> c_int {
    unimplemented!();
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub unsafe extern "C" fn fegetround() -> c_int {
    let mut fpcr: u32;
    asm!("mrs {}, fpcr", out(reg) fpcr);
    (fpcr >> 22) as c_int
}

#[cfg(target_arch = "riscv64")]
#[no_mangle]
pub unsafe extern "C" fn fegetround() -> c_int {
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    (fcsr >> 5) as c_int
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn fegetround() -> c_int {
    unimplemented!();
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub unsafe extern "C" fn fesetround(round: c_int) -> c_int {
    let mut fpcr: u32;
    asm!("mrs {}, fpcr", out(reg) fpcr);
    fpcr &= !(0b11 << 22);
    fpcr |= (round as u32) << 22;
    asm!("msr fpcr, {}", in(reg) fpcr);
    0
}

#[cfg(target_arch = "riscv64")]
#[no_mangle]
pub unsafe extern "C" fn fesetround(round: c_int) -> c_int {
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    fcsr &= !(0b111 << 5);
    fcsr |= (round as u32) << 5;
    asm!("fscsr {}", in(reg) fcsr);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn fesetround(round: c_int) -> c_int {
    unimplemented!();
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub unsafe extern "C" fn fegetenv(envp: *mut fenv_t) -> c_int {
    asm!("mrs {}, fpcr", out(reg) (*envp).__fpcr);
    asm!("mrs {}, fpsr", out(reg) (*envp).__fpsr);
    0
}

#[cfg(target_arch = "riscv64")]
#[no_mangle]
pub unsafe extern "C" fn fegetenv(envp: *mut fenv_t) -> c_int {
    asm!("frcsr {}", out(reg) (*envp).__fcsr);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn fegetenv(envp: *mut fenv_t) -> c_int {
    unimplemented!();
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
#[no_mangle]
pub unsafe extern "C" fn feholdexcept(envp: *mut fenv_t) -> c_int {
    fegetenv(envp);
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    fcsr &= !FE_ALL_EXCEPT as u32;
    asm!("fscsr {}", in(reg) fcsr);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn feholdexcept(envp: *mut fenv_t) -> c_int {
    unimplemented!();
}

#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub unsafe extern "C" fn fesetenv(envp: *const fenv_t) -> c_int {
    asm!("msr fpcr, {}", in(reg) (*envp).__fpcr);
    asm!("msr fpsr, {}", in(reg) (*envp).__fpsr);
    0
}

#[cfg(target_arch = "riscv64")]
#[no_mangle]
pub unsafe extern "C" fn fesetenv(envp: *const fenv_t) -> c_int {
    asm!("fscsr {}", in(reg) (*envp).__fcsr);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn fesetenv(envp: *const fenv_t) -> c_int {
    unimplemented!();
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
#[no_mangle]
pub unsafe extern "C" fn feupdateenv(envp: *const fenv_t) -> c_int {
    let mut fcsr: u32;
    asm!("frcsr {}", out(reg) fcsr);
    fesetenv(envp);
    feraiseexcept(fcsr as c_int);
    0
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
#[no_mangle]
pub unsafe extern "C" fn feupdateenv(envp: *const fenv_t) -> c_int {
    unimplemented!();
}
