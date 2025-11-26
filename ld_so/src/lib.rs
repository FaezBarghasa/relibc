// ld_so/src/lib.rs
//! The Relibc User-Space Dynamic Linker (ld.so)
//!
//! This crate implements the ELF dynamic linking process for Redox and Linux.
//! It is a freestanding Pure Rust library that bootstraps itself.

#![no_std]
#![feature(naked_functions)]
//#![feature(link_args)]
#![allow(clippy::missing_safety_doc)]

extern crate alloc;

// --- Core Modules ---
pub mod callbacks;
pub mod debug;
pub mod dso;
pub mod header; // Assumes symlink or path to main relibc header module
pub mod linker;
pub mod reloc;
pub mod start;
pub mod tcb;
pub mod tls;
pub mod verify;

// --- Linux Parity Modules ---
pub mod gnu_hash;
pub mod linux_parity;
pub mod versioning;

use core::arch::global_asm;
use crate::linker::Linker;
use crate::dso::DSO;
use crate::debug;

// Define the entry point _start.
#[cfg(target_arch = "x86_64")]
global_asm!(
    ".globl _start",
    "_start:",
    "mov rdi, rsp", // Arg 1: Stack Pointer
    "call linker_entry",
    "ud2"
);

#[cfg(target_arch = "aarch64")]
global_asm!(
    ".globl _start",
    "_start:",
    "mov x0, sp",
    "bl linker_entry",
    "brk #1"
);

#[cfg(target_arch = "riscv64")]
global_asm!(
    ".globl _start",
    "_start:",
    "mv a0, sp",
    "call linker_entry",
    "unimp"
);

/// The Rust Entry Point.
#[no_mangle]
pub unsafe extern "C" fn linker_entry(sp: *const usize) -> ! {
    let main_dso = DSO::new_executable(sp);
    let load_base = main_dso.base_addr;

    if let Err(e) = verify::verify_self_integrity(load_base) {
        debug::dprintln(e);
        core::intrinsics::abort();
    }
    if let Err(e) = verify::verify_bss_relocations() {
        debug::dprintln(e);
        core::intrinsics::abort();
    }

    // Find envp to parse Tunables
    // Stack Layout: [Argc] [Argv...] [0] [Envp...]
    let argc = *sp;
    let argv = sp.add(1);
    let envp = argv.add(argc + 1) as *const *const i8;

    // 3. Initialize Linker with Env vars
    let mut linker = Linker::new(envp);

    linker.link(main_dso);

    enter_entry_point(sp, linker.get_entry_point());
}

#[cfg(target_arch = "x86_64")]
unsafe fn enter_entry_point(sp: *const usize, entry: usize) -> ! {
    core::arch::asm!(
        "mov rsp, {0}",
        "jmp {1}",
        in(reg) sp,
        in(reg) entry,
        options(noreturn)
    )
}

#[cfg(target_arch = "aarch64")]
unsafe fn enter_entry_point(sp: *const usize, entry: usize) -> ! {
    core::arch::asm!(
        "mov sp, {0}",
        "br {1}",
        in(reg) sp,
        in(reg) entry,
        options(noreturn)
    )
}

#[cfg(target_arch = "riscv64")]
unsafe fn enter_entry_point(sp: *const usize, entry: usize) -> ! {
    core::arch::asm!(
        "mv sp, {0}",
        "jr {1}",
        in(reg) sp,
        in(reg) entry,
        options(noreturn)
    )
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    core::intrinsics::abort()
}

#[no_mangle]
pub extern "C" fn __dso_handle() {}

#[no_mangle]
pub extern "C" fn __rust_alloc(size: usize, align: usize) -> *mut u8 {
    core::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn __rust_dealloc(_ptr: *mut u8, _size: usize, _align: usize) {}

#[no_mangle]
pub extern "C" fn __rust_realloc(_ptr: *mut u8, _old_size: usize, _align: usize, _new_size: usize) -> *mut u8 {
    core::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn __rust_alloc_zeroed(_size: usize, _align: usize) -> *mut u8 {
    core::ptr::null_mut()
}