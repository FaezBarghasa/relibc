use core::{mem, slice};
use redox_rt::proc::FdGuard;
use syscall;

use crate::{
    error::{Errno, Result, ResultExt},
    header::{
        errno::{self, EINVAL},
        fcntl, termios,
    },
    platform::{self, Pal, Sys, types::*},
};

use super::winsize;

pub const TCGETS: c_ulong = 0x5401;
pub const TCSETS: c_ulong = 0x5402;
pub const TCSETSW: c_ulong = 0x5403;
pub const TCSETSF: c_ulong = 0x5404;

pub const TCSBRK: c_ulong = 0x5409;
pub const TCXONC: c_ulong = 0x540A;
pub const TCFLSH: c_ulong = 0x540B;

pub const TIOCSCTTY: c_ulong = 0x540E;
pub const TIOCGPGRP: c_ulong = 0x540F;
pub const TIOCSPGRP: c_ulong = 0x5410;

pub const TIOCGWINSZ: c_ulong = 0x5413;
pub const TIOCSWINSZ: c_ulong = 0x5414;

pub const FIONREAD: c_ulong = 0x541B;

pub const FIONBIO: c_ulong = 0x5421;

pub const TIOCSPTLCK: c_ulong = 0x4004_5431;
pub const TIOCGPTLCK: c_ulong = 0x8004_5439;

pub const SIOCATMARK: c_ulong = 0x8905;

// TODO: some of the structs passed as T have padding bytes, so casting to a byte slice is UB

fn dup_read<T>(fd: c_int, name: &str, t: &mut T) -> syscall::Result<usize> {
    let dup = FdGuard::new(syscall::dup(fd as usize, name.as_bytes())?);

    let size = mem::size_of::<T>();

    let bytes = dup.read(unsafe { slice::from_raw_parts_mut(t as *mut T as *mut u8, size) })?;

    Ok(bytes / size)
}

// FIXME: unsound
fn dup_write<T>(fd: c_int, name: &str, t: &T) -> Result<usize> {
    let dup = FdGuard::new(syscall::dup(fd as usize, name.as_bytes())?);

    let size = mem::size_of::<T>();

    let bytes = dup.write(unsafe { slice::from_raw_parts(t as *const T as *const u8, size) })?;

    Ok(bytes / size)
}

unsafe fn ntsync_ioctl(fd: c_int, request: c_ulong, out: *mut c_void) -> Result<c_int> {
    use crate::header::sys_sync::*;
    let request_val = request as u32;
    match request_val {
        0xc0084e80 => { // NTSYNC_IOC_CREATE_SEM
            let args = &mut *(out as *mut ntsync_sem_args);
            ntsync_create_sem(args.count, args.max)
        }
        0xc0084e84 => { // NTSYNC_IOC_CREATE_MUTEX
            let args = &mut *(out as *mut ntsync_mutex_args);
            ntsync_create_mutex(args.owner, args.count)
        }
        0xc0084e88 => { // NTSYNC_IOC_CREATE_EVENT
            let args = &mut *(out as *mut ntsync_event_args);
            ntsync_create_event(args.manual != 0, args.signaled != 0)
        }
        0xc0044e81 => { // NTSYNC_IOC_SEM_POST
            let release_count = *(out as *const u32);
            let prev = ntsync_sem_post(fd, release_count)?;
            *(out as *mut u32) = prev;
            Ok(0)
        }
        0xc0084e85 => { // NTSYNC_IOC_MUTEX_UNLOCK
            let args = &mut *(out as *mut ntsync_mutex_args);
            let (prev_owner, prev_count) = ntsync_mutex_unlock(fd, args.owner)?;
            args.owner = prev_owner;
            args.count = prev_count;
            Ok(0)
        }
        0xc0044e89 => { // NTSYNC_IOC_EVENT_SET
            let prev = ntsync_event_set(fd)?;
            *(out as *mut u32) = prev;
            Ok(0)
        }
        0xc0044e8a => { // NTSYNC_IOC_EVENT_RESET
            let prev = ntsync_event_reset(fd)?;
            *(out as *mut u32) = prev;
            Ok(0)
        }
        0xc0044e8b => { // NTSYNC_IOC_EVENT_PULSE
            let prev = ntsync_event_pulse(fd)?;
            *(out as *mut u32) = prev;
            Ok(0)
        }
        0x80084e8c => { // NTSYNC_IOC_SEM_READ
            let args = &mut *(out as *mut ntsync_sem_args);
            let (count, max) = ntsync_sem_read(fd)?;
            args.count = count;
            args.max = max;
            Ok(0)
        }
        0x80084e8d => { // NTSYNC_IOC_MUTEX_READ
            let args = &mut *(out as *mut ntsync_mutex_args);
            let (owner, count) = ntsync_mutex_read(fd)?;
            args.owner = owner;
            args.count = count;
            Ok(0)
        }
        0x80084e8e => { // NTSYNC_IOC_EVENT_READ
            let args = &mut *(out as *mut ntsync_event_args);
            let (manual, signaled) = ntsync_event_read(fd)?;
            args.manual = if manual { 1 } else { 0 };
            args.signaled = signaled;
            Ok(0)
        }
        0xc0204e82 => { // NTSYNC_IOC_WAIT_ANY
            let args = &mut *(out as *mut ntsync_wait_args);
            let handles = core::slice::from_raw_parts(args.objs as *const c_int, args.count as usize);
            let index = ntsync_wait_any(handles, args.timeout)?;
            args.index = index;
            Ok(0)
        }
        0xc0204e86 => { // NTSYNC_IOC_WAIT_ALL
            let args = &mut *(out as *mut ntsync_wait_args);
            let handles = core::slice::from_raw_parts(args.objs as *const c_int, args.count as usize);
            ntsync_wait_all(handles, args.timeout)?;
            Ok(0)
        }
        _ => Err(Errno(EINVAL)),
    }
}

unsafe fn ioctl_inner(fd: c_int, request: c_ulong, out: *mut c_void) -> Result<c_int> {
    if crate::header::sys_sync::is_ntsync_fd(fd) {
        return ntsync_ioctl(fd, request, out);
    }
    match request {
        FIONBIO => {
            let mut flags = Sys::fcntl(fd, fcntl::F_GETFL, 0)?;
            flags = if *(out as *mut c_int) == 0 {
                flags & !fcntl::O_NONBLOCK
            } else {
                flags | fcntl::O_NONBLOCK
            };
            Sys::fcntl(fd, fcntl::F_SETFL, flags as c_ulonglong)?;
        }
        TCGETS => {
            let termios = &mut *(out as *mut termios::termios);
            dup_read(fd, "termios", termios)?;
        }
        // TODO: give these different behaviors
        TCSETS | TCSETSW | TCSETSF => {
            let termios = &*(out as *const termios::termios);
            dup_write(fd, "termios", termios)?;
        }
        TCFLSH => {
            let queue = out as c_int;
            dup_write(fd, "flush", &queue)?;
        }
        TIOCSCTTY => {
            eprintln!("TODO: ioctl TIOCSCTTY");
        }
        TIOCGPGRP => {
            let pgrp = &mut *(out as *mut pid_t);
            dup_read(fd, "pgrp", pgrp)?;
        }
        TIOCSPGRP => {
            let pgrp = &*(out as *const pid_t);
            dup_write(fd, "pgrp", pgrp)?;
        }
        TIOCGWINSZ => {
            let winsize = &mut *(out as *mut winsize);
            dup_read(fd, "winsize", winsize)?;
        }
        TIOCSWINSZ => {
            let winsize = &*(out as *const winsize);
            dup_write(fd, "winsize", winsize)?;
        }
        TIOCGPTLCK => {
            eprintln!("TODO: ioctl TIOCGPTLCK");
        }
        TIOCSPTLCK => {
            eprintln!("TODO: ioctl TIOCSPTLCK");
        }
        TCSBRK => {
            eprintln!("TODO: ioctl TCSBRK");
        }
        TCXONC => {
            eprintln!("TODO: ioctl TCXONC");
        }
        SIOCATMARK => {
            eprintln!("TODO: ioctl SIOCATMARK");
        }
        _ => {
            return Err(Errno(EINVAL));
        }
    }
    Ok(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioctl(fd: c_int, request: c_ulong, out: *mut c_void) -> c_int {
    ioctl_inner(fd, request, out).or_minus_one_errno()
}
