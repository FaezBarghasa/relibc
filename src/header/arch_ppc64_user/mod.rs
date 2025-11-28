use crate::platform::types::*;

// Placeholder for PowerPC 64-bit user structs
#[repr(C)]
pub struct user_regs_struct {
    // TODO: Define actual PPC64 registers
    pub gpr: [c_ulong; 32],
}

pub type elf_greg_t = c_ulong;
pub type elf_gregset_t = [c_ulong; 48];
pub type elf_fpregset_t = [c_double; 33];

#[repr(C)]
pub struct user {
    pub regs: user_regs_struct,
}
