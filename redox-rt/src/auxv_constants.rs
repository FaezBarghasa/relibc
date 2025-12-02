// Redox-specific auxiliary vector constants
// These are the values used when exec'ing a new process on Redox

pub const AT_REDOX_INITIAL_CWD_PTR: usize = 32;
pub const AT_REDOX_INITIAL_CWD_LEN: usize = 33;
pub const AT_REDOX_INHERITED_SIGIGNMASK: usize = 34;
#[cfg(target_pointer_width = "32")]
pub const AT_REDOX_INHERITED_SIGIGNMASK_HI: usize = 35;
pub const AT_REDOX_INHERITED_SIGPROCMASK: usize = 36;
#[cfg(target_pointer_width = "32")]
pub const AT_REDOX_INHERITED_SIGPROCMASK_HI: usize = 37;
pub const AT_REDOX_INITIAL_DEFAULT_SCHEME_PTR: usize = 38;
pub const AT_REDOX_INITIAL_DEFAULT_SCHEME_LEN: usize = 39;
pub const AT_REDOX_UMASK: usize = 40;
pub const AT_REDOX_PROC_FD: usize = 41;
pub const AT_REDOX_THR_FD: usize = 42;
