//! fenv implementation for Redox, following http://pubs.opengroup.org/onlinepubs/7908799/xsh/fenv.h.html

use crate::platform::types::*;
use core::arch::asm;

#[repr(C)]
#[cfg(target_arch = "x86")]
pub struct fenv_t {
    __control: u16,
    __reserved1: u16,
    __status: u16,
    __reserved2: [u8; 22],
}

#[repr(C)]
#[cfg(target_arch = "x86_64")]
pub struct fenv_t {
    pub __control: u16,
    pub __reserved1: u16,
    pub __status: u16,
    pub __reserved2: [u8; 22], // x87 state is 28 bytes total
    pub __mxcsr: u32,
}

#[repr(C)]
#[cfg(target_arch = "aarch64")]
pub struct fenv_t {
    pub __value: u64, // Combined FPCR (high 32) and FPSR (low 32)
}

#[repr(C)]
#[cfg(target_arch = "riscv64")]
pub struct fenv_t {
    __fcsr: u64,
}

#[repr(C)]
pub struct fexcept_t {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    __except: u16,
    #[cfg(target_arch = "aarch64")]
    __except: u64,
    #[cfg(target_arch = "riscv64")]
    __except: u64,
}

// Architecture-specific constants
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod consts {
    use super::*;
    pub const FE_INVALID: c_int = 0x01;
    pub const FE_DENORMAL: c_int = 0x02; // Not in standard POSIX but present in hardware
    pub const FE_DIVBYZERO: c_int = 0x04;
    pub const FE_OVERFLOW: c_int = 0x08;
    pub const FE_UNDERFLOW: c_int = 0x10;
    pub const FE_INEXACT: c_int = 0x20;
    pub const FE_ALL_EXCEPT: c_int =
        FE_DIVBYZERO | FE_DENORMAL | FE_INEXACT | FE_INVALID | FE_OVERFLOW | FE_UNDERFLOW;

    pub const FE_TONEAREST: c_int = 0x0000;
    pub const FE_DOWNWARD: c_int = 0x0400;
    pub const FE_UPWARD: c_int = 0x0800;
    pub const FE_TOWARDZERO: c_int = 0x0c00;
}

#[cfg(any(target_arch = "aarch64"))]
pub mod consts {
    use super::*;
    pub const FE_INVALID: c_int = 0x01;
    pub const FE_DIVBYZERO: c_int = 0x02;
    pub const FE_OVERFLOW: c_int = 0x04;
    pub const FE_UNDERFLOW: c_int = 0x08;
    pub const FE_INEXACT: c_int = 0x10;
    pub const FE_ALL_EXCEPT: c_int =
        FE_DIVBYZERO | FE_INEXACT | FE_INVALID | FE_OVERFLOW | FE_UNDERFLOW;

    pub const FE_TONEAREST: c_int = 0x0;
    pub const FE_UPWARD: c_int = 0x1;
    pub const FE_DOWNWARD: c_int = 0x2;
    pub const FE_TOWARDZERO: c_int = 0x3;
}

#[cfg(any(target_arch = "riscv64"))]
pub mod consts {
    use super::*;
    // RISC-V FCSR:
    // 0: NX (Inexact)
    // 1: UF (Underflow)
    // 2: OF (Overflow)
    // 3: DZ (DivByZero)
    // 4: NV (Invalid)
    pub const FE_INEXACT: c_int = 0x01;
    pub const FE_UNDERFLOW: c_int = 0x02;
    pub const FE_OVERFLOW: c_int = 0x04;
    pub const FE_DIVBYZERO: c_int = 0x08;
    pub const FE_INVALID: c_int = 0x10;
    pub const FE_ALL_EXCEPT: c_int =
        FE_DIVBYZERO | FE_INEXACT | FE_INVALID | FE_OVERFLOW | FE_UNDERFLOW;

    pub const FE_TONEAREST: c_int = 0x0; // RNE
    pub const FE_TOWARDZERO: c_int = 0x1; // RTZ
    pub const FE_DOWNWARD: c_int = 0x2; // RDN
    pub const FE_UPWARD: c_int = 0x3; // RUP
}

pub use consts::*;

#[cfg(target_arch = "x86")]
pub const FE_DFL_ENV: *const fenv_t = &fenv_t {
    __control: 0x037F,
    __reserved1: 0,
    __status: 0,
    __reserved2: [0; 22],
};
#[cfg(target_arch = "x86_64")]
pub const FE_DFL_ENV: *const fenv_t = &fenv_t {
    __control: 0x037F,
    __reserved1: 0,
    __status: 0,
    __reserved2: [0; 22],
    __mxcsr: 0x1F80,
};
#[cfg(target_arch = "aarch64")]
pub const FE_DFL_ENV: *const fenv_t = &fenv_t { __value: 0 };
#[cfg(target_arch = "riscv64")]
pub const FE_DFL_ENV: *const fenv_t = &fenv_t { __fcsr: 0 };

#[no_mangle]
pub unsafe extern "C" fn feclearexcept(excepts: c_int) -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        // Store x87 state
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
        asm!("fnstsw ax", out("ax") status);

        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);

        (*flagp).__except = ((status as u16) | mxcsr as u16) & (excepts as u16);
    }
    #[cfg(target_arch = "x86")]
    {
        let status: u16;
        asm!("fnstsw ax", out("ax") status);
        (*flagp).__except = (status & (excepts as u16));
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
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status |= excepts as u16;
        asm!("fldenv [{}]", in(reg) &fenv);
        asm!("fwait"); // Trigger x87 exception

        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);
        mxcsr |= excepts as u32;
        asm!("ldmxcsr [{}]", in(reg) &mxcsr); // Trigger SSE exception
    }
    #[cfg(target_arch = "x86")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status |= excepts as u16;
        asm!("fldenv [{}]", in(reg) &fenv);
        asm!("fwait");
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
        fenv.__status |= (*flagp).__except & (excepts as u16);
        asm!("fldenv [{}]", in(reg) &fenv);

        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);
        mxcsr &= !(excepts as u32);
        mxcsr |= (*flagp).__except as u32 & (excepts as u32);
        asm!("ldmxcsr [{}]", in(reg) &mxcsr);
    }
    #[cfg(target_arch = "x86")]
    {
        let mut fenv: fenv_t = core::mem::MaybeUninit::uninit().assume_init();
        asm!("fnstenv [{}]", in(reg) &mut fenv);
        fenv.__status &= !(excepts as u16);
        fenv.__status |= (*flagp).__except & (excepts as u16);
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
        return ((mxcsr >> 3) & 3) as c_int; // x86_64 uses SSE shift 3, x87 uses 10
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
        mxcsr &= !(3 << 3); // SSE shift 3
        mxcsr |= (round as u32) << 3;
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
        let fpcr: u64;
        let fpsr: u64;
        asm!("mrs {}, fpcr", out(reg) fpcr);
        asm!("mrs {}, fpsr", out(reg) fpsr);
        (*envp).__value = fpsr | (fpcr << 32);
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
        asm!("fnstenv [{}]", in(reg) envp);
        asm!("fclex");

        asm!("stmxcsr [{}]", in(reg) &mut (*envp).__mxcsr);
        let mut mxcsr: u32 = (*envp).__mxcsr;
        // Mask exceptions: bits 7-12
        mxcsr |= 0x1F80;
        // Clear flags: bits 0-5
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
        let fpcr: u64;
        let fpsr: u64;
        asm!("mrs {}, fpcr", out(reg) fpcr);
        asm!("mrs {}, fpsr", out(reg) fpsr);
        (*envp).__value = fpsr | (fpcr << 32);

        let mut new_fpcr = fpcr;
        // Mask exceptions (bits 8-12, 15?).
        // openlibm: _ENABLE_MASK = FE_ALL_EXCEPT << 8.
        // FE_ALL_EXCEPT = 0x1F.
        // So bits 8-12.
        // 0 = Masked (Disabled)? No.
        // openlibm: __r &= ~(_ENABLE_MASK); -> Clearing bits 8-12.
        // AArch64 FPCR: 0=Untrapped(Masked), 1=Trapped(Enabled).
        // So clearing bits disables traps (masks them).
        new_fpcr &= !((FE_ALL_EXCEPT as u64) << 8);
        asm!("msr fpcr, {}", in(reg) new_fpcr);

        let mut new_fpsr = fpsr;
        new_fpsr &= !(FE_ALL_EXCEPT as u64);
        asm!("msr fpsr, {}", in(reg) new_fpsr);
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
        let fpcr = (*envp).__value >> 32;
        let fpsr = (*envp).__value as u32 as u64;
        asm!("msr fpcr, {}", in(reg) fpcr);
        asm!("msr fpsr, {}", in(reg) fpsr);
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
        let status: u16;
        asm!("fnstsw ax", out("ax") status);
        let mut mxcsr: u32 = 0;
        asm!("stmxcsr [{}]", in(reg) &mut mxcsr);

        let current_excepts = ((status as u32) | mxcsr) & (FE_ALL_EXCEPT as u32);

        fesetenv(envp);

        feraiseexcept(current_excepts as c_int);
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
        let current_excepts = fpsr & (FE_ALL_EXCEPT as u64);

        fesetenv(envp);

        feraiseexcept(current_excepts as c_int);
    }
    #[cfg(target_arch = "riscv64")]
    {
        let mut fcsr: u64;
        asm!("frcsr {}", out(reg) fcsr);
        let current_excepts = fcsr & (FE_ALL_EXCEPT as u64);

        fesetenv(envp);

        feraiseexcept(current_excepts as c_int);
    }
    0
}
