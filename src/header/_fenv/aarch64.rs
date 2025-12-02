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
