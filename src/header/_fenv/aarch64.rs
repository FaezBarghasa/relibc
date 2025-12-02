use crate::platform::types::*;

#[inline(always)]
pub unsafe fn fegetround() -> c_int {
    let mut fpcr: u64;
    asm!("mrs {}, fpcr", out(reg) fpcr);
    ((fpcr >> 22) & 3) as c_int
}

#[inline(always)]
pub unsafe fn fesetround(round: c_int) -> c_int {
    let mut fpcr: u64;
    asm!("mrs {}, fpcr", out(reg) fpcr);
    fpcr &= !0xC00000;
    fpcr |= ((round as u64) & 3) << 22;
    asm!("msr fpcr, {}", in(reg) fpcr);
    0
}

#[inline(always)]
pub unsafe fn feclearexcept(excepts: c_int) -> c_int {
    let mut fpsr: u64;
    asm!("mrs {}, fpsr", out(reg) fpsr);
    fpsr &= !(excepts as u64 & 0x3F);
    asm!("msr fpsr, {}", in(reg) fpsr);
    0
}

#[inline(always)]
pub unsafe fn feraiseexcept(excepts: c_int) -> c_int {
    let mut fpsr: u64;
    asm!("mrs {}, fpsr", out(reg) fpsr);
    fpsr |= excepts as u64 & 0x3F;
    asm!("msr fpsr, {}", in(reg) fpsr);
    0
}

#[inline(always)]
pub unsafe fn fetestexcept(excepts: c_int) -> c_int {
    let mut fpsr: u64;
    asm!("mrs {}, fpsr", out(reg) fpsr);
    (fpsr as c_int) & excepts
}

#[inline(always)]
pub unsafe fn fegetenv(envp: *mut fenv_t) -> c_int {
    asm!("mrs {}, fpcr", out(reg) (*envp).cw);
    0
}

#[inline(always)]
pub unsafe fn feholdexcept(envp: *mut fenv_t) -> c_int {
    let mut fpcr: u64;
    asm!("mrs {}, fpcr", out(reg) fpcr);
    (*envp).cw = fpcr;
    fpcr &= !0x3F;
    asm!("msr fpcr, {}", in(reg) fpcr);
    0
}

#[inline(always)]
pub unsafe fn fesetenv(envp: *const fenv_t) -> c_int {
    asm!("msr fpcr, {}", in(reg) (*envp).cw);
    0
}

#[inline(always)]
pub unsafe fn feupdateenv(envp: *const fenv_t) -> c_int {
    let mut fpsr: u64;
    asm!("mrs {}, fpsr", out(reg) fpsr);
    asm!("msr fpcr, {}", in(reg) (*envp).cw);
    fpsr |= (*envp).cw & 0x3F;
    asm!("msr fpsr, {}", in(reg) fpsr);
    0
}

#[inline(always)]
pub unsafe fn fegetexceptflag(flagp: *mut fexcept_t, excepts: c_int) -> c_int {
    let mut fpsr: u64;
    asm!("mrs {}, fpsr", out(reg) fpsr);
    *flagp = fpsr & (excepts as u64 & 0x3F);
    0
}

#[inline(always)]
pub unsafe fn fesetexceptflag(flagp: *const fexcept_t, excepts: c_int) -> c_int {
    let mut fpsr: u64;
    asm!("mrs {}, fpsr", out(reg) fpsr);
    fpsr &= !(excepts as u64 & 0x3F);
    fpsr |= *flagp & (excepts as u64 & 0x3F);
    asm!("msr fpsr, {}", in(reg) fpsr);
    0
}
