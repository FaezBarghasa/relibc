use crate::platform::types::*;

// Placeholder for x86 (32-bit) user structs
#[repr(C)]
pub struct user_fpregs_struct {
    pub cwd: c_long,
    pub swd: c_long,
    pub twd: c_long,
    pub fip: c_long,
    pub fcs: c_long,
    pub foo: c_long,
    pub fos: c_long,
    pub st_space: [c_long; 20],
}

#[repr(C)]
pub struct user_regs_struct {
    pub ebx: c_long,
    pub ecx: c_long,
    pub edx: c_long,
    pub esi: c_long,
    pub edi: c_long,
    pub ebp: c_long,
    pub eax: c_long,
    pub xds: c_long,
    pub xes: c_long,
    pub xfs: c_long,
    pub xgs: c_long,
    pub orig_eax: c_long,
    pub eip: c_long,
    pub xcs: c_long,
    pub eflags: c_long,
    pub esp: c_long,
    pub xss: c_long,
}

pub type elf_greg_t = c_ulong;
pub type elf_gregset_t = [c_ulong; 17];
pub type elf_fpregset_t = user_fpregs_struct;

#[repr(C)]
pub struct user {
    pub regs: user_regs_struct,
    // ...
}
