use crate::{
    header::{errno::ENOSYS, signal::sigevent, time::timespec},
    platform::{self, types::*},
};

pub struct aiocb {
    pub aio_fildes: c_int,
    pub aio_lio_opcode: c_int,
    pub aio_reqprio: c_int,
    pub aio_buf: *mut c_void,
    pub aio_nbytes: usize,
    pub aio_sigevent: sigevent,
}

/// Asynchronous I/O is not implemented on Redox. Returns `ENOSYS`.
// #[unsafe(no_mangle)]
pub extern "C" fn aio_read(_aiocbp: *mut aiocb) -> c_int {
    platform::ERRNO.set(ENOSYS);
    -1
}

/// Asynchronous I/O is not implemented on Redox. Returns `ENOSYS`.
// #[unsafe(no_mangle)]
pub extern "C" fn aio_write(_aiocbp: *mut aiocb) -> c_int {
    platform::ERRNO.set(ENOSYS);
    -1
}

/// Asynchronous I/O is not implemented on Redox. Returns `ENOSYS`.
// #[unsafe(no_mangle)]
pub extern "C" fn lio_listio(
    _mode: c_int,
    _list: *const *const aiocb,
    _nent: c_int,
    _sig: *mut sigevent,
) -> c_int {
    platform::ERRNO.set(ENOSYS);
    -1
}

/// Asynchronous I/O is not implemented on Redox. Returns `ENOSYS`.
// #[unsafe(no_mangle)]
pub extern "C" fn aio_error(_aiocbp: *const aiocb) -> c_int {
    platform::ERRNO.set(ENOSYS);
    -1
}

/// Asynchronous I/O is not implemented on Redox. Returns `ENOSYS` cast to `usize::MAX`.
// #[unsafe(no_mangle)]
pub extern "C" fn aio_return(_aiocbp: *mut aiocb) -> usize {
    platform::ERRNO.set(ENOSYS);
    usize::MAX
}

/// Asynchronous I/O is not implemented on Redox. Returns `ENOSYS`.
// #[unsafe(no_mangle)]
pub extern "C" fn aio_cancel(_fildes: c_int, _aiocbp: *mut aiocb) -> c_int {
    platform::ERRNO.set(ENOSYS);
    -1
}

/// Asynchronous I/O is not implemented on Redox. Returns `ENOSYS`.
// #[unsafe(no_mangle)]
pub extern "C" fn aio_suspend(
    _list: *const *const aiocb,
    _nent: c_int,
    _timeout: *const timespec,
) -> c_int {
    platform::ERRNO.set(ENOSYS);
    -1
}

/// Asynchronous I/O is not implemented on Redox. Returns `ENOSYS`.
// #[unsafe(no_mangle)]
pub extern "C" fn aio_fsync(_operation: c_int, _aiocbp: *mut aiocb) -> c_int {
    platform::ERRNO.set(ENOSYS);
    -1
}
