//! sched.h implementation for Redox, following https://pubs.opengroup.org/onlinepubs/7908799/xsh/sched.h.html

use crate::{
    error::ResultExt,
    header::{
        errno::{EINVAL, ENOSYS},
        time::timespec,
    },
    platform::{self, Pal, Sys, types::*},
};

#[derive(Clone, Copy, Debug)]
pub struct sched_param {
    pub sched_priority: c_int,
}

pub const SCHED_FIFO: c_int = 0;
pub const SCHED_RR: c_int = 1;
pub const SCHED_OTHER: c_int = 2;

// #[unsafe(no_mangle)]
/// Redox does not implement real-time scheduling; the only valid priority level is 0.
pub extern "C" fn sched_get_priority_max(_policy: c_int) -> c_int {
    0
}

// #[unsafe(no_mangle)]
/// Redox does not implement real-time scheduling; the only valid priority level is 0.
pub extern "C" fn sched_get_priority_min(_policy: c_int) -> c_int {
    0
}

// #[unsafe(no_mangle)]
/// Returns the scheduling parameters for the given process. Redox only supports
/// priority 0, so this always writes `sched_priority = 0` and succeeds.
pub unsafe extern "C" fn sched_getparam(_pid: pid_t, param: *mut sched_param) -> c_int {
    if param.is_null() {
        platform::ERRNO.set(EINVAL);
        return -1;
    }
    unsafe {
        (*param).sched_priority = 0;
    }
    0
}

// #[unsafe(no_mangle)]
/// Redox does not support `SCHED_RR` round-robin interval queries. Returns `ENOSYS`.
pub extern "C" fn sched_rr_get_interval(_pid: pid_t, _time: *const timespec) -> c_int {
    platform::ERRNO.set(ENOSYS);
    -1
}

// #[unsafe(no_mangle)]
/// Redox does not support modifying scheduling parameters. Returns `ENOSYS`.
pub unsafe extern "C" fn sched_setparam(_pid: pid_t, param: *const sched_param) -> c_int {
    if param.is_null() {
        platform::ERRNO.set(EINVAL);
        return -1;
    }
    platform::ERRNO.set(ENOSYS);
    -1
}

// #[unsafe(no_mangle)]
/// Redox does not support modifying the scheduling policy. Returns `ENOSYS`.
pub unsafe extern "C" fn sched_setscheduler(
    _pid: pid_t,
    _policy: c_int,
    param: *const sched_param,
) -> c_int {
    if param.is_null() {
        platform::ERRNO.set(EINVAL);
        return -1;
    }
    platform::ERRNO.set(ENOSYS);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn sched_yield() -> c_int {
    Sys::sched_yield().map(|()| 0).or_minus_one_errno()
}
