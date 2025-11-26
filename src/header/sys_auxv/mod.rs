//! `sys/auxv.h` implementation.
//!
//! Non-POSIX, see <https://www.man7.org/linux/man-pages/man3/getauxval.3.html>.

use crate::platform::types::*;

pub use crate::platform::auxv_defs::*;

/// See <https://www.man7.org/linux/man-pages/man3/getauxval.3.html>.
#[unsafe(no_mangle)]
pub extern "C" fn getauxval(t: c_ulong) -> c_ulong {
    if let Some(auxvs) = unsafe { crate::platform::AUXVS.unsafe_ref() } {
        if let Some(v) = crate::platform::get_auxv(auxvs, t as usize) {
            v as c_ulong
        } else {
            0
        }
    } else {
        0
    }
}
