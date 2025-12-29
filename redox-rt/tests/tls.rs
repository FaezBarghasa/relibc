#![feature(asm)]

use std::thread;
use redox_rt::{Tcb, RtTcb, initialize_freestanding};
use redox_rt::proc::FdGuard;

#[test]
fn test_tls() {
    #[cfg(target_arch = "aarch64")]
    use redox_rt::arch::aarch64::{get_thread_local, set_thread_local};
    #[cfg(target_arch = "riscv64")]
    use redox_rt::arch::riscv64::{get_thread_local, set_thread_local};

    assert_eq!(get_thread_local(), 42);
    set_thread_local(100);
    assert_eq!(get_thread_local(), 100);

    let handle = thread::spawn(|| {
        assert_eq!(get_thread_local(), 42);
        set_thread_local(200);
        assert_eq!(get_thread_local(), 200);
    });

    handle.join().unwrap();

    assert_eq!(get_thread_local(), 100);

    let handle = thread::spawn(|| {
        assert_eq!(get_thread_local(), 42);
    });

    handle.join().unwrap();
}

#[test]
fn test_fork_exec_simulation() {
    #[cfg(target_arch = "aarch64")]
    use redox_rt::arch::aarch64::{get_thread_local, set_thread_local, deactivate_tcb};
    #[cfg(target_arch = "riscv64")]
    use redox_rt::arch::riscv64::{get_thread_local, set_thread_local, deactivate_tcb};

    // 1. Simulate fork
    set_thread_local(300);

    let handle = thread::spawn(|| {
        // In the "child" thread, the TLS should be initialized to its default value
        assert_eq!(get_thread_local(), 42);
        set_thread_local(400);
        assert_eq!(get_thread_local(), 400);
    });

    handle.join().unwrap();

    // The parent's TLS should be unaffected
    assert_eq!(get_thread_local(), 300);

    // 2. Simulate exec
    unsafe {
        // This is a very rough simulation. In a real exec, the address space would be replaced.
        let tcb = RtTcb::current();
        let thr_fd = tcb.thread_fd().dup(&[]).unwrap().to_upper().unwrap();
        deactivate_tcb(&thr_fd).unwrap();

        // Re-initialize the TCB to simulate a new process
        let new_thr_fd = thr_fd.dup(&[]).unwrap().to_upper().unwrap();
        initialize_freestanding(new_thr_fd);
    }

    // After the "exec", the TLS should be in a clean state
    assert_eq!(get_thread_local(), 42);
}
