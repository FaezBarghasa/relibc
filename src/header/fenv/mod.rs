//! fenv implementation for Redox, following http://pubs.opengroup.org/onlinepubs/7908799/xsh/fenv.h.html

use crate::platform::types::*;
use core::arch::asm;

#[repr(C)]
pub struct fenv_t {
    #[cfg(target_arch = "aarch64")]
    __fpcr: u64,
    #[cfg(target_arch = "aarch64")]
    __fpsr: u64,
    #[cfg(target_arch = "riscv64")]
    __fcsr: u64,
}

#[repr(C)]
pub struct fexcept_t {
    #[cfg(target_arch = "aarch64")]
    __except: u64,
    #[cfg(target_arch = "riscv64")]
    __except: u64,
}

pub const FE_DIVBYZERO: c_int = 1;
pub const FE_INEXACT: c_int = 2;
pub const FE_INVALID: c_int = 4;
pub const FE_OVERFLOW: c_int = 8;
pub const FE_UNDERFLOW: c_int = 16;
pub const FE_ALL_EXCEPT: c_int = FE_DIVBYZERO | FE_INEXACT | FE_INVALID | FE_OVERFLOW | FE_UNDERFLOW;

pub const FE_DOWNWARD: c_int = 1;
pub const FE_TONEAREST: c_int = 2;
pub const FE_TOWARDZERO: c_int = 3;
pub const FE_UPWARD: c_int = 4;

#[no_mangle]
pub unsafe extern "C" fn feclearexcept(excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        fpsr &= !(excepts as u64);
        asm!("msr fpsr, {}", in(reg) fpsr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        fcsr &= !(excepts as u64);
        asm!("fscsr {}", in(reg) fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fegetexceptflag(flagp: *mut fexcept_t, excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        (*flagp).__except = fpsr & (excepts as u64);
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        (*flagp).__except = fcsr & (excepts as u64);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn feraiseexcept(excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        fpsr |= excepts as u64;
        asm!("msr fpsr, {}", in(reg) fpsr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        fcsr |= excepts as u64;
        asm!("fscsr {}", in(reg) fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fesetexceptflag(flagp: *const fexcept_t, excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        fpsr &= !(excepts as u64);
        fpsr |= (*flagp).__except & (excepts as u64);
        asm!("msr fpsr, {}", in(reg) fpsr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        fcsr &= !(excepts as u64);
        fcsr |= (*flagp).__except & (excepts as u64);
        asm!("fscsr {}", in(reg) fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fetestexcept(excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        return (fpsr as c_int) & excepts;
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        return (fcsr as c_int) & excepts;
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
    {
        unimplemented!();
    }
}

#[no_mangle]
pub unsafe extern "C" fn fegetround() -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpcr: u64;
        asm!("mrs {}, fpcr", out(reg) fpcr);
        return ((fpcr >> 22) & 3) as c_int;
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        return ((fcsr >> 5) & 7) as c_int;
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
    {
        unimplemented!();
    }
}

#[no_mangle]
pub unsafe extern "C" fn fesetround(round: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpcr: u64;
        asm!("mrs {}, fpcr", out(reg) fpcr);
        fpcr &= !(3 << 22);
        fpcr |= (round as u64) << 22;
        asm!("msr fpcr, {}", in(reg) fpcr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        fcsr &= !(7 << 5);
        fcsr |= (round as u64) << 5;
        asm!("fscsr {}", in(reg) fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fegetenv(envp: *mut fenv_t) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        asm!("mrs {}, fpcr", out(reg) (*envp).__fpcr);
        asm!("mrs {}, fpsr", out(reg) (*envp).__fpsr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        asm!("frcsr {}", out(reg) (*envp).__fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn feholdexcept(envp: *mut fenv_t) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        asm!("mrs {}, fpcr", out(reg) (*envp).__fpcr);
        asm!("mrs {}, fpsr", out(reg) (*envp).__fpsr);
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        fpsr &= !FE_ALL_EXCEPT as u64;
        asm!("msr fpsr, {}", in(reg) fpsr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        asm!("frcsr {}", out(reg) (*envp).__fcsr);
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        fcsr &= !FE_ALL_EXCEPT as u64;
        asm!("fscsr {}", in(reg) fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fesetenv(envp: *const fenv_t) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        asm!("msr fpcr, {}", in(reg) (*envp).__fpcr);
        asm!("msr fpsr, {}", in(reg) (*envp).__fpsr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        asm!("fscsr {}", in(reg) (*envp).__fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn feupdateenv(envp: *const fenv_t) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        asm!("msr fpcr, {}", in(reg) (*envp).__fpcr);
        asm!("msr fpsr, {}", in(reg) (*envp).__fpsr | (fpsr & FE_ALL_EXCEPT as u64));
    }
    #[cfg(target_.arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        asm!("fscsr {}", in(reg) (*envp).__fcsr | (fcsr & FE_ALL_EXCEPT as u64));
    }
    0
}
