use crate::platform::types::*;

// Placeholder for MIPS user structs
#[repr(C)]
pub struct user_regs_struct {
    // TODO: Define actual MIPS registers
    pub r1: c_ulong,
}

pub type elf_greg_t = c_ulong;
pub type elf_gregset_t = [c_ulong; 32];
pub type elf_fpregset_t = [c_ulong; 32];

#[repr(C)]
pub struct user {
    pub regs: user_regs_struct,
}
