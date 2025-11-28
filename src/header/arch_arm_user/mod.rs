use crate::platform::types::*;

// Placeholder for ARM (32-bit) user structs
#[repr(C)]
pub struct user_regs_struct {
    // TODO: Define actual ARM registers
    pub r0: c_ulong,
    // ...
}

pub type elf_greg_t = c_ulong;
pub type elf_gregset_t = [c_ulong; 18]; // Example size
pub type elf_fpregset_t = [c_ulong; 32]; // Example

#[repr(C)]
pub struct user {
    pub regs: user_regs_struct,
    // ...
}
