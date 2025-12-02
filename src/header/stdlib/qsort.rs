use crate::platform::types::*;
use core::{mem, slice};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn qsort(
    base: *mut c_void,
    nel: size_t,
    width: size_t,
    comp: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
) {
    if let Some(comp) = comp {
        let mut slice = slice::from_raw_parts_mut(base as *mut u8, nel * width);
        slice.sort_unstable_by(|a, b| {
            let res = comp(a.as_ptr() as *const c_void, b.as_ptr() as *const c_void);
            res.cmp(&0)
        });
    }
}
