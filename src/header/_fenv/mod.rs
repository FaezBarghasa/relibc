//! fenv implementation for Redox, following http://pubs.opengroup.org/onlinepubs/7908799/xsh/fenv.h.html

use crate::platform::types::*;
use core::arch::asm;

#[repr(C)]
#[cfg(target_arch = "x86")]
pub struct fenv_t {
    __control: u16,
    __status: u16,
    __reserved: [u32; 5],
}

#[repr(C)]
#[cfg(target_arch = "x86_64")]
pub struct fenv_t {
    pub __control: u16,
    pub __status: u16,
    pub __reserved_x87: [u8; 24],
    pub __mxcsr: u32,
}

#[repr(C)]
#[cfg(target_arch = "aarch64")]
pub struct fenv_t {
    __fpcr: u64,
    __fpsr: u64,
}

#[repr(C)]
#[cfg(target_arch = "riscv64")]
pub struct fenv_t {
    __fcsr: u64,
}

#[repr(C)]
pub struct fexcept_t {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    __except: u32,
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
pub const FE_ALL_EXCEPT: c_int =
    FE_DIVBYZERO | FE_INEXACT | FE_INVALID | FE_OVERFLOW | FE_UNDERFLOW;

pub const FE_DOWNWARD: c_int = 1;
pub const FE_TONEAREST: c_int = 2;
pub const FE_TOWARDZERO: c_int = 3;
pub const FE_UPWARD: c_int = 4;

#[cfg(target_arch = "x86")]
pub const FE_DFL_ENV: *const fenv_t = &fenv_t {
    __control: 0x37F,
    __status: 0,
    __reserved: [0; 5],
};
#[cfg(target_arch = "x86_64")]
pub const FE_DFL_ENV: *const fenv_t = &fenv_t {
    __control: 0x37F,
    __status: 0,
    __reserved_x87: [0; 24],
    __mxcsr: 0x1F80,
};
#[cfg(target_arch = "aarch64")]
pub const FE_DFL_ENV: *const fenv_t = &fenv_t {
    __fpcr: 0,
    __fpsr: 0,
};
#[cfg(target_arch = "riscv64")]
pub const FE_DFL_ENV: *const fenv_t = &fenv_t { __fcsr: 0 };

#[no_mangle]
pub unsafe extern "C" fn feclearexcept(excepts: c_int) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        // Store x87 state - fnstenv masks exceptions, so we must reload
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status &= !(excepts as u16);
        asm!("fldenv [{}]", in(reg) &fenv);

        // Store SSE state
        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);
        mxcsr &= !(excepts as u32);
        asm!("ldmxcsr [{}]", in(reg) &mxcsr);
    }
    #[cfg(target_arch = "x86")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status &= !(excepts as u16);
        asm!("fldenv [{}]", in(reg) &fenv);
    }
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
    #[cfg(target_arch = "x86_64")]
    {
        let status: u16;
        // Use fnstsw ax to read status without altering mask state
        asm!("fnstsw ax", out("ax") status);

        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);

        (*flagp).__except = ((status as u32) | mxcsr) & (excepts as u32);
    }
    #[cfg(target_arch = "x86")]
    {
        let status: u16;
        asm!("fnstsw ax", out("ax") status);
        (*flagp).__except = (status & (excepts as u16)) as u32;
    }
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
    #[cfg(target_arch = "x86_64")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        // x87: fnstenv -> modify status -> fldenv (triggers trap if unmasked)
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status |= excepts as u16;
        asm!("fldenv [{}]", in(reg) &fenv);

        // SSE: stmxcsr -> modify status -> ldmxcsr (triggers trap if unmasked)
        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);
        mxcsr |= excepts as u32;
        asm!("ldmxcsr [{}]", in(reg) &mxcsr);
    }
    #[cfg(target_arch = "x86")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status |= excepts as u16;
        asm!("fldenv [{}]", in(reg) &fenv);
    }
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
    #[cfg(target_arch = "x86_64")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status &= !(excepts as u16);
        fenv.__status |= (*flagp).__except as u16 & (excepts as u16);
        asm!("fldenv [{}]", in(reg) &fenv);

        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);
        mxcsr &= !(excepts as u32);
        mxcsr |= (*flagp).__except & (excepts as u32);
        asm!("ldmxcsr [{}]", in(reg) &mxcsr);
    }
    #[cfg(target_arch = "x86")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status &= !(excepts as u16);
        fenv.__status |= (*flagp).__except as u16 & (excepts as u16);
        asm!("fldenv [{}]", in(reg) &fenv);
    }
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
    #[cfg(target_arch = "x86_64")]
    {
        let status: u16;
        asm!("fnstsw ax", out("ax") status);
        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);
        return ((status as c_int) | (mxcsr as c_int)) & excepts;
    }
    #[cfg(target_arch = "x86")]
    {
        let status: u16;
        asm!("fnstsw ax", out("ax") status);
        return (status as c_int) & excepts;
    }
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
    #[cfg(not(any(
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        unimplemented!();
    }
}

#[no_mangle]
pub unsafe extern "C" fn fegetround() -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);
        return ((mxcsr >> 13) & 3) as c_int;
    }
    #[cfg(target_arch = "x86")]
    {
        let mut control: u16;
        asm!("fnstcw [{}]", in(reg) &mut control);
        return ((control >> 10) & 3) as c_int;
    }
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
    #[cfg(not(any(
        target_arch = "x86",
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    {
        unimplemented!();
    }
}

#[no_mangle]
pub unsafe extern "C" fn fesetround(round: c_int) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let mut control: u16 = 0;
        asm!("fnstcw [{}]", in(reg) &mut control);
        control &= !(3 << 10);
        control |= (round as u16) << 10;
        asm!("fldcw [{}]", in(reg) &control);

        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);
        mxcsr &= !(3 << 13);
        mxcsr |= (round as u32) << 13;
        asm!("ldmxcsr [{}]", in(reg) &mxcsr);
    }
    #[cfg(target_arch = "x86")]
    {
        let mut control: u16;
        asm!("fnstcw [{}]", in(reg) &mut control);
        control &= !(3 << 10);
        control |= (round as u16) << 10;
        asm!("fldcw [{}]", in(reg) &control);
    }
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
    #[cfg(target_arch = "x86_64")]
    {
        // fnstenv masks all exceptions. We must restore the mask state.
        asm!("fnstenv [{}]", in(reg) envp);
        asm!("fldenv [{}]", in(reg) envp);
        asm!("stmxcsr [{}]", in(reg) &mut (*envp).__mxcsr);
    }
    #[cfg(target_arch = "x86")]
    {
        asm!("fnstenv [{}]", in(reg) envp);
        asm!("fldenv [{}]", in(reg) envp);
    }
    #[cfg(target_arch = "aarch64")]
    {
        asm!("mrs {}, fpcr", out(reg)(*envp).__fpcr);
        asm!("mrs {}, fpsr", out(reg)(*envp).__fpsr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        asm!("frcsr {}", out(reg)(*envp).__fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn feholdexcept(envp: *mut fenv_t) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        // x87: fnstenv saves state AND masks all exceptions.
        asm!("fnstenv [{}]", in(reg) envp);
        // Clear x87 exception flags
        asm!("fclex");

        // SSE
        asm!("stmxcsr [{}]", in(reg) &mut (*envp).__mxcsr);
        let mut mxcsr: u32 = (*envp).__mxcsr;
        // Broadcast mask to all exception mask bits (7-12)
        mxcsr |= 0x1F80;
        // Clear all exception status bits (0-5)
        mxcsr &= !0x3F;
        asm!("ldmxcsr [{}]", in(reg) &mxcsr);
    }
    #[cfg(target_arch = "x86")]
    {
        asm!("fnstenv [{}]", in(reg) envp);
        asm!("fclex");
    }
    #[cfg(target_arch = "aarch64")]
    {
        asm!("mrs {}, fpcr", out(reg)(*envp).__fpcr);
        asm!("mrs {}, fpsr", out(reg)(*envp).__fpsr);
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        fpsr &= !FE_ALL_EXCEPT as u64;
        asm!("msr fpsr, {}", in(reg) fpsr);
    }
    #[cfg(target_arch = "riscv64")]
    {
        asm!("frcsr {}", out(reg)(*envp).__fcsr);
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        fcsr &= !FE_ALL_EXCEPT as u64;
        asm!("fscsr {}", in(reg) fcsr);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fesetenv(envp: *const fenv_t) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        asm!("fldenv [{}]", in(reg) envp);
        asm!("ldmxcsr [{}]", in(reg) &(*envp).__mxcsr);
    }
    #[cfg(target_arch = "x86")]
    {
        asm!("fldenv [{}]", in(reg) envp);
    }
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
    #[cfg(target_arch = "x86_64")]
    {
        // 1. Save current exceptions
        let status: u16;
        asm!("fnstsw ax", out("ax") status);
        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);

        let current_excepts = (status as c_int | mxcsr as c_int) & FE_ALL_EXCEPT;

        // 2. Install new environment
        asm!("fldenv [{}]", in(reg) envp);
        asm!("ldmxcsr [{}]", in(reg) &(*envp).__mxcsr);

        // 3. Raise saved exceptions
        feraiseexcept(current_excepts);
    }
    #[cfg(target_arch = "x86")]
    {
        let status: u16;
        asm!("fnstsw ax", out("ax") status);
        let current_excepts = (status as c_int) & FE_ALL_EXCEPT;

        asm!("fldenv [{}]", in(reg) envp);

        feraiseexcept(current_excepts);
    }
    #[cfg(target_arch = "aarch64")]
    {
        let mut fpsr: u64;
        asm!("mrs {}, fpsr", out(reg) fpsr);
        asm!("msr fpcr, {}", in(reg) (*envp).__fpcr);
        asm!("msr fpsr, {}", in(reg) (*envp).__fpsr | (fpsr & FE_ALL_EXCEPT as u64));
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        asm!("fscsr {}", in(reg) (*envp).__fcsr | (fcsr & FE_ALL_EXCEPT as u64));
    }
    0
}
