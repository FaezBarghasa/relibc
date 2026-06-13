use core::ffi::c_void;

#[repr(C)]
pub struct Stack {
    pub argc: isize,
    pub argv: *mut *mut u8,
    pub base: *mut usize,
    pub len: usize,
}

pub static mut OPTIMIZED_MEMCPY: unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> *mut c_void = default_memcpy;

pub static mut HAS_AVX512: bool = false;
pub static mut HAS_AMX: bool = false;
pub static mut HAS_AVXVNNI: bool = false;

unsafe extern "C" fn default_memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    core::ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, n);
    dest
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe extern "C" fn avx512_memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let mut i = 0;
    let d = dest as *mut u8;
    let s = src as *const u8;
    while i + 64 <= n {
        use core::arch::x86_64::{_mm512_loadu_si512, _mm512_storeu_si512};
        let chunk = _mm512_loadu_si512(s.add(i).cast());
        _mm512_storeu_si512(d.add(i).cast(), chunk);
        i += 64;
    }
    if i < n {
        core::ptr::copy_nonoverlapping(s.add(i), d.add(i), n - i);
    }
    dest
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "amx-tile")]
pub unsafe fn amx_zero_tiles() {
    // AMX tile configuration/zeroing placeholder
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avxvnni")]
pub unsafe fn avxvnni_dot_product(a: &[i8], b: &[i8]) -> i32 {
    // AVX-VNNI optimized path placeholder
    0
}

pub unsafe fn init_cpuid_features() {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::x86_64::__cpuid_count;
        let res7_0 = __cpuid_count(7, 0);
        let has_avx512f = (res7_0.ebx & (1 << 16)) != 0;
        let has_amx_tile = (res7_0.edx & (1 << 24)) != 0;

        let res7_1 = __cpuid_count(7, 1);
        let has_avx_vnni = (res7_1.eax & (1 << 4)) != 0;

        if has_avx512f {
            OPTIMIZED_MEMCPY = avx512_memcpy;
        }

        HAS_AVX512 = has_avx512f;
        HAS_AMX = has_amx_tile;
        HAS_AVXVNNI = has_avx_vnni;
    }
}
