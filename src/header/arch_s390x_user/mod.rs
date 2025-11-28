use crate::platform::types::*;

// Placeholder for S390x user structs
#[repr(C)]
pub struct user_regs_struct {
    // TODO: Define actual S390x registers
    pub psw: [c_ulong; 2],
}

pub type elf_greg_t = c_ulong;
pub type elf_gregset_t = [c_ulong; 27];
pub type elf_fpregset_t = [c_ulong; 16];

#[repr(C)]
pub struct user {
    pub regs: user_regs_struct,
}
