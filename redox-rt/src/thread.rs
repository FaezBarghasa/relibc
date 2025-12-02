use core::mem::size_of;

use syscall::{Map, Result, MapFlags, PROT_READ, PROT_WRITE};

use crate::{RtTcb, Tcb, arch::*, proc::*, signal::{setup_sighandler, tmp_disable_signals}, static_proc_info, tcb_activate};

/// Spawns a new context sharing the same address space as the current one (i.e. a new thread).
pub unsafe fn rlct_clone_impl(
    new_sp: *mut usize,
    new_stack_base: *mut (),
    new_stack_size: usize,
    new_tls: *mut (),
    new_dtv: *mut (),
    new_dtv_len: usize,
) -> Result<FdGuardUpper> {
    let proc_info = static_proc_info();
    assert!(proc_info.has_proc_fd);
    let cur_proc_fd = unsafe { proc_info.proc_fd.assume_init_ref() };

    let cur_thr_fd = RtTcb::current().thread_fd();
    let new_thr_fd = cur_proc_fd.dup(b"new-thread")?.to_upper().unwrap();

    // Inherit existing address space
    {
        let cur_addr_space_fd = cur_thr_fd.dup(b"addrspace")?;
        let new_addr_space_sel_fd = new_thr_fd.dup(b"current-addrspace")?;

        let buf = create_set_addr_space_buf(
            cur_addr_space_fd.as_raw_fd(),
            __relibc_internal_rlct_clone_ret as usize,
            new_sp as usize,
        );
        new_addr_space_sel_fd.write(&buf)?;
    }

    // Inherit reference to file table
    {
        let cur_filetable_fd = cur_thr_fd.dup(b"filetable")?;
        let new_filetable_sel_fd = new_thr_fd.dup(b"current-filetable")?;

        new_filetable_sel_fd.write(&usize::to_ne_bytes(cur_filetable_fd.as_raw_fd()))?;
    }

    // Since the signal handler is not yet initialized, signals specifically targeting the thread
    // (relibc is only required to implement thread-specific signals that already originate from
    // the same process) will be discarded. Process-specific signals will ignore this new thread,
    // until it has initialized its own signal handler.

    let tcb_guard = MmapGuard::map(
        cur_thr_fd.dup(b"addrspace")?.to_upper().unwrap(),
        &Map {
            size: syscall::PAGE_SIZE,
            address: 0,
            offset: 0,
            flags: PROT_READ | PROT_WRITE,
        },
    )?;
    let tcb = &mut *(tcb_guard.addr() as *mut Tcb);
    tcb.tcb_ptr = tcb;
    tcb.tcb_len = tcb_guard.len();
    tcb.platform_specific.stack_base = new_stack_base;
    tcb.platform_specific.stack_size = new_stack_size;
    tcb.platform_specific.tls_dtv = new_dtv;
    tcb.platform_specific.tls_dtv_len = new_dtv_len;
    tcb.platform_specific.tls_static_base = new_tls;
    tcb.os_specific
        .thr_fd
        .get()
        .write(Some(new_thr_fd.dup(b"").unwrap().to_upper().unwrap()));

    setup_sighandler(&tcb.os_specific, false);

    // Unblock context.
    let start_fd = new_thr_fd.dup(b"start")?;
    start_fd.write(&[0])?;

    Ok(new_thr_fd)
}

pub unsafe fn exit_this_thread() -> ! {
    let _guard = tmp_disable_signals();

    let tcb = Tcb::current().unwrap();
    // TODO: modify interface so it writes directly to the thread fd?
    let status_fd = tcb.os_specific.thread_fd().dup(b"status").unwrap();

    let _ = unsafe {
        syscall::funmap(
            tcb.platform_specific.stack_base as usize,
            tcb.platform_specific.stack_size,
        )
    };
    let _ = unsafe { syscall::funmap(tcb as *const Tcb as usize, tcb.tcb_len) };

    let mut buf = [0; size_of::<usize>() * 3];
    plain::slice_from_mut_bytes(&mut buf)
        .unwrap()
        .copy_from_slice(&[usize::MAX, 0, 0]);
    // TODO: SYS_CALL w/CONSUME
    status_fd.write(&buf).unwrap();
    unreachable!()
}
