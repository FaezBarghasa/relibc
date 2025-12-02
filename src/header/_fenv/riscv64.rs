use crate::platform::types::*;

#[inline(always)]
pub unsafe fn fegetround() -> c_int {
    let mut fcsr: u64;
    asm!("frcsr {}", out(reg) fcsr);
    ((fcsr >> 5) & 3) as c_int
}

#[inline(always)]
pub unsafe fn fesetround(round: c_int) -> c_int {
    let mut fcsr: u64;
    asm!("frcsr {}", out(reg) fcsr);
    fcsr &= !0xE0;
    fcsr |= ((round as u64) & 3) << 5;
    asm!("fscsr {}", in(reg) fcsr);
    0
}
