//! fenv.h implementation for Redox, following
//! http://pubs.opengroup.org/onlinepubs/9699919799/basedefs/fenv.h.html

use crate::platform::types::*;

#[cfg(target_arch = "aarch64")]
#[path = "aarch64.rs"]
mod arch;

pub const FE_ALL_EXCEPT: c_int = 0x3F;
pub const FE_TONEAREST: c_int = 0;

pub type fexcept_t = u64;

#[repr(C)]
pub struct fenv_t {
    pub cw: u64,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn feclearexcept(excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::feclearexcept(excepts)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fegetenv(envp: *mut fenv_t) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::fegetenv(envp)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fegetexceptflag(flagp: *mut fexcept_t, excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::fegetexceptflag(flagp, excepts)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fegetround() -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::fegetround()
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        FE_TONEAREST
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn feholdexcept(envp: *mut fenv_t) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::feholdexcept(envp)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn feraiseexcept(excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::feraiseexcept(excepts)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fesetenv(envp: *const fenv_t) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::fesetenv(envp)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fesetexceptflag(flagp: *const fexcept_t, excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::fesetexceptflag(flagp, excepts)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fesetround(round: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::fesetround(round)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fetestexcept(excepts: c_int) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::fetestexcept(excepts)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn feupdateenv(envp: *const fenv_t) -> c_int {
    #[cfg(target_arch = "aarch64")]
    {
        arch::feupdateenv(envp)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        unimplemented!();
    }
}
