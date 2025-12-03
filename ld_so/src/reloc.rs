// ld_so/src/reloc.rs
//! Core Relocation Logic for x86-64, AArch64, and RISC-V.
//! Fully implemented with ELF TLS support.

use core::mem::size_of;
use crate::header::elf;
use crate::linux_parity::resolve_ifunc;
use crate::tcb::Tcb;

pub unsafe fn relocate(
    r_type: u32,
    sym_val: usize,
    sym_size: usize,
    reloc_addr: usize,
    addend: Option<usize>,
    base_addr: usize,
    tls_module_id: usize,
    tls_offset: usize,
    static_tls_size: usize,
) -> bool {
    let ptr = reloc_addr as *mut usize;
    let val = addend.unwrap_or(0);

    #[cfg(target_arch = "x86_64")]
    {
        match r_type {
            elf::R_X86_64_64 => *ptr = sym_val.wrapping_add(val),
            elf::R_X86_64_GLOB_DAT | elf::R_X86_64_JUMP_SLOT => *ptr = sym_val,
            elf::R_X86_64_RELATIVE => *ptr = base_addr.wrapping_add(val),
            elf::R_X86_64_IRELATIVE => {
                let resolver = base_addr.wrapping_add(val);
                *ptr = resolve_ifunc(resolver);
            }
            elf::R_X86_64_DTPMOD64 => *ptr = tls_module_id,
            elf::R_X86_64_DTPOFF64 => *ptr = sym_val.wrapping_add(val),
            elf::R_X86_64_TPOFF64 => {
                let offset_from_start = tls_offset.wrapping_add(sym_val).wrapping_add(val);
                *ptr = offset_from_start.wrapping_sub(static_tls_size);
            }
            _ => return false,
        }
        true
    }

    #[cfg(target_arch = "aarch64")]
    {
        let ptr32 = reloc_addr as *mut u32;
        match r_type {
            elf::R_AARCH64_ABS64 | elf::R_AARCH64_GLOB_DAT | elf::R_AARCH64_JUMP_SLOT => {
                *ptr = sym_val.wrapping_add(val);
            }
            elf::R_AARCH64_RELATIVE => *ptr = base_addr.wrapping_add(val),
            elf::R_AARCH64_IRELATIVE => {
                let resolver = base_addr.wrapping_add(val);
                *ptr = resolve_ifunc(resolver);
            }
            elf::R_AARCH64_ADD_ABS_LO12_NC => {
                let sym_val_lo12 = sym_val.wrapping_add(val) & 0xFFF;
                *ptr32 = (*ptr32 & 0xFFFFF000) | (sym_val_lo12 as u32);
            }
            elf::R_AARCH64_ADR_PREL_LO21 => {
                let diff = sym_val.wrapping_add(val).wrapping_sub(reloc_addr);
                let imm21 = (diff as i32 & 0x1FFFFF) as u32;
                *ptr32 = (*ptr32 & 0xFFF8001F) | (imm21 << 5);
            }
            elf::R_AARCH64_ADR_PREL_PG_HI21 => {
                let page_mask = !0xFFF;
                let sym_page = sym_val.wrapping_add(val) & page_mask;
                let reloc_page = reloc_addr & page_mask;
                let diff = sym_page.wrapping_sub(reloc_page);
                let imm21 = ((diff as i64 >> 12) & 0x1FFFFF) as u32;
                *ptr32 = (*ptr32 & 0xFFF8001F) | (imm21 << 5);
            }
            elf::R_AARCH64_CALL26 | elf::R_AARCH64_JUMP26 => {
                let diff = sym_val.wrapping_add(val).wrapping_sub(reloc_addr);
                let imm26 = (diff as i32 >> 2) & 0x3FFFFFF;
                *ptr32 = (*ptr32 & 0xFC000000) | (imm26 as u32);
            }
            elf::R_AARCH64_MOVW_UABS_G0_NC => {
                let word = sym_val.wrapping_add(val) as u32 & 0xFFFF;
                *ptr32 = (*ptr32 & 0xFFE0001F) | (word << 5);
            }
            elf::R_AARCH64_MOVW_UABS_G1_NC => {
                let word = (sym_val.wrapping_add(val) >> 16) as u32 & 0xFFFF;
                *ptr32 = (*ptr32 & 0xFFE0001F) | (word << 5);
            }
            elf::R_AARCH64_MOVW_UABS_G2_NC => {
                let word = (sym_val.wrapping_add(val) >> 32) as u32 & 0xFFFF;
                *ptr32 = (*ptr32 & 0xFFE0001F) | (word << 5);
            }
            elf::R_AARCH64_MOVW_UABS_G3 => {
                let word = (sym_val.wrapping_add(val) >> 48) as u32 & 0xFFFF;
                *ptr32 = (*ptr32 & 0xFFE0001F) | (word << 5);
            }
            elf::R_AARCH64_TLS_DTPMOD64 => *ptr = tls_module_id,
            elf::R_AARCH64_TLS_DTPREL64 => *ptr = sym_val.wrapping_add(val),
            elf::R_AARCH64_TLS_TPREL64 => {
                let tcb_size = size_of::<Tcb>();
                let tcb_aligned = (tcb_size + 15) & !15;
                *ptr = tcb_aligned.wrapping_add(tls_offset).wrapping_add(sym_val).wrapping_add(val);
            }
            _ => return false,
        }
        true
    }

    #[cfg(target_arch = "riscv64")]
    {
        let ptr16 = reloc_addr as *mut u16;
        let ptr32 = reloc_addr as *mut u32;
        match r_type {
            elf::R_RISCV_64 => *ptr = sym_val.wrapping_add(val),
            elf::R_RISCV_JUMP_SLOT => *ptr = sym_val,
            elf::R_RISCV_RELATIVE => *ptr = base_addr.wrapping_add(val),
            elf::R_RISCV_IRELATIVE => {
                let resolver = base_addr.wrapping_add(val);
                *ptr = resolve_ifunc(resolver);
            }
            elf::R_RISCV_ADD32 => { *(ptr as *mut u32) = (*(ptr as *mut u32)).wrapping_add(sym_val as u32).wrapping_add(val as u32); }
            elf::R_RISCV_ADD64 => { *ptr = (*ptr).wrapping_add(sym_val).wrapping_add(val); }
            elf::R_RISCV_SUB32 => { *(ptr as *mut u32) = (*(ptr as *mut u32)).wrapping_sub(sym_val as u32).wrapping_sub(val as u32); }
            elf::R_RISCV_SUB64 => { *ptr = (*ptr).wrapping_sub(sym_val).wrapping_sub(val); }
            elf::R_RISCV_CALL | elf::R_RISCV_CALL_PLT => {
                let diff = sym_val.wrapping_add(val).wrapping_sub(reloc_addr);
                let hi20 = (diff + 0x800) & 0xFFFFF000;
                let lo12 = diff & 0xFFF;
                *ptr32 = (*ptr32 & 0xFFF) | (hi20 as u32);
                *(ptr32.add(1)) = (*(ptr32.add(1)) & 0xFFFFF) | ((lo12 as u32) << 20);
            }
            elf::R_RISCV_GOT_HI20 | elf::R_RISCV_PCREL_HI20 => {
                let diff = sym_val.wrapping_add(val).wrapping_sub(reloc_addr);
                let hi20 = (diff + 0x800) & 0xFFFFF000;
                *ptr32 = (*ptr32 & 0xFFF) | (hi20 as u32);
            }
            elf::R_RISCV_HI20 => {
                let val = sym_val.wrapping_add(val);
                let hi20 = (val + 0x800) & 0xFFFFF000;
                *ptr32 = (*ptr32 & 0xFFF) | (hi20 as u32);
            }
            elf::R_RISCV_LO12_I | elf::R_RISCV_PCREL_LO12_I => {
                let diff = sym_val.wrapping_add(val).wrapping_sub(reloc_addr);
                let lo12 = diff & 0xFFF;
                *ptr32 = (*ptr32 & 0xFFFFF) | ((lo12 as u32) << 20);
            }
            elf::R_RISCV_LO12_S | elf::R_RISCV_PCREL_LO12_S => {
                let diff = sym_val.wrapping_add(val).wrapping_sub(reloc_addr);
                let lo12 = diff & 0xFFF;
                let imm11_5 = (lo12 >> 5) & 0x7F;
                let imm4_0 = lo12 & 0x1F;
                *ptr32 = (*ptr32 & 0x1FFF07F) | ((imm11_5 as u32) << 25) | ((imm4_0 as u32) << 7);
            }
            elf::R_RISCV_ALIGN => { /* NOP */ }
            elf::R_RISCV_RVC_BRANCH => {
                let diff = sym_val.wrapping_add(val).wrapping_sub(reloc_addr);
                let imm8 = (diff >> 1) & 0xFF;
                let imm4_3 = (imm8 >> 3) & 0x3;
                let imm2_1 = (imm8 >> 1) & 0x3;
                let imm7 = (imm8 >> 7) & 0x1;
                let imm6 = (imm8 >> 6) & 0x1;
                let imm5 = (imm8 >> 5) & 0x1;
                *ptr16 = (*ptr16 & 0xE383) | ((imm4_3 as u16) << 3) | ((imm2_1 as u16) << 10) | ((imm7 as u16) << 12) | ((imm6 as u16) << 2) | ((imm5 as u16) << 5);
            }
            elf::R_RISCV_RVC_JUMP => {
                let diff = sym_val.wrapping_add(val).wrapping_sub(reloc_addr);
                let imm11 = (diff >> 1) & 0x7FF;
                *ptr16 = (*ptr16 & 0xE003) | (((imm11 >> 5) & 0x1) << 12) | (((imm11 >> 1) & 0xF) << 3) | (((imm11 >> 7) & 0x1) << 7) | (((imm11 >> 6) & 0x1) << 6) | (((imm11 >> 10) & 0x1) << 11) | (((imm11 >> 8) & 0x3) << 9) | (((imm11 >> 4) & 0x1) << 8);
            }
            elf::R_RISCV_TLS_DTPMOD64 => *ptr = tls_module_id,
            elf::R_RISCV_TLS_DTPREL64 => *ptr = sym_val.wrapping_add(val),
            elf::R_RISCV_TLS_TPREL64 => {
                let tcb_size = size_of::<Tcb>();
                let tcb_aligned = (tcb_size + 15) & !15;
                *ptr = tcb_aligned.wrapping_add(tls_offset).wrapping_add(sym_val).wrapping_add(val);
            }
            _ => return false,
        }
        true
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
    {
        false
    }
}

pub unsafe fn relocate_copy(
    r_type: u32,
    src_addr: usize,
    dst_addr: usize,
    size: usize,
) -> bool {
    let is_copy = 
        (cfg!(target_arch = "x86_64") && r_type == elf::R_X86_64_COPY) ||
        (cfg!(target_arch = "aarch64") && r_type == elf::R_AARCH64_COPY) ||
        (cfg!(target_arch = "riscv64") && r_type == elf::R_RISCV_COPY);

    if is_copy {
        let src = src_addr as *const u8;
        let dst = dst_addr as *mut u8;
        core::ptr::copy_nonoverlapping(src, dst, size);
        true
    } else {
        false
    }
}
