use core::{cell::SyncUnsafeCell, ptr::NonNull};

use syscall::error::*;

use crate::{proc::FdGuardUpper, protocol::RtSigInfo, signal::SigStack, Tcb};

// Setup a stack starting from the very end of the address space, and then growing downwards.
pub const STACK_TOP: usize = 1 << 47;
pub const STACK_SIZE: usize = 1024 * 1024;
// TODO: shouldn't this be an atomic?
pub static PROC_FD: SyncUnsafeCell<usize> = SyncUnsafeCell::new(usize::MAX);

#[derive(Debug, Default)]
#[repr(C)]
pub struct SigArea {
    pub tmp_rip: usize,
    pub tmp_rsp: usize,
    pub tmp_rax: usize,
    pub tmp_rdx: usize,
    pub tmp_rdi: usize,
    pub tmp_rsi: usize,
    pub tmp_r8: usize,
    pub tmp_r10: usize,
    pub tmp_r12: usize,
    pub tmp_rt_inf: RtSigInfo,
    pub tmp_id_inf: u64,

    pub altstack_top: usize,
    pub altstack_bottom: usize,
    pub disable_signals_depth: u64,
    pub last_sig_was_restart: bool,
    pub last_sigstack: Option<NonNull<SigStack>>,
}

#[repr(C, align(16))]
#[derive(Debug, Default)]
pub struct ArchIntRegs {
    pub ymm_upper: [u128; 16],
    pub fxsave: [u128; 29],
    pub fs_base: u64,
    pub gs_base: u64,
}

#[derive(Debug, Default)]
#[repr(C)]
pub struct TcbExtension {
    pub self_ptr: *mut Tcb,
    pub stack_base: *mut usize,
    pub stack_size: usize,
    pub tls_dtv: *mut usize,
    pub tls_dtv_len: usize,
    pub tls_static_base: *mut usize,
    pub my_thread_local: usize,
}

pub fn get_thread_local() -> usize {
    let tcb: *mut Tcb;
    unsafe {
        core::arch::asm!("mov {}, fs:[0]", out(reg) tcb);
        (*tcb).platform_specific.my_thread_local
    }
}

pub fn set_thread_local(value: usize) {
    let tcb: *mut Tcb;
    unsafe {
        core::arch::asm!("mov {}, fs:[0]", out(reg) tcb);
        (*tcb).platform_specific.my_thread_local = value;
    }
}

pub unsafe fn deactivate_tcb(_fd: &FdGuardUpper) -> Result<()> {
    // TODO
    Ok(())
}

pub unsafe fn arch_pre(
    _stack: &mut crate::signal::SigStack,
    _area: &mut SigArea,
) -> crate::signal::PosixStackt {
    // TODO
    unsafe { core::mem::zeroed() }
}

pub unsafe fn manually_enter_trampoline() {
    // TODO
    core::arch::asm!("ud2");
}

pub unsafe fn current_sp() -> usize {
    let sp: usize;
    core::arch::asm!("mov {}, rsp", out(reg) sp);
    sp
}
