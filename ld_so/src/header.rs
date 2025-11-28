// Minimal ELF header definitions required for ld_so
pub mod elf {
    // Basic ELF types (using usize for simplicity)
    pub type Addr = usize;
    pub type Off = usize;
    pub type Half = u16;
    pub type Word = u32;
    pub type Sword = i32;
    pub type Xword = u64;
    pub type Sxword = i64;

    // Constants for auxiliary vector types
    pub const AT_NULL: usize = 0;
    pub const AT_PHDR: usize = 3;
    pub const AT_PHNUM: usize = 5;
    pub const AT_ENTRY: usize = 9;

    // Program header types
    pub const PT_NULL: u32 = 0;
    pub const PT_LOAD: u32 = 1;
    pub const PT_DYNAMIC: u32 = 2;

    // Dynamic tag constants
    pub const DT_NULL: i64 = 0;

    // ELF relocation types (x86_64)
    pub const R_X86_64_64: u32 = 1;
    pub const R_X86_64_COPY: u32 = 5;
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
    pub const R_X86_64_RELATIVE: u32 = 8;
    pub const R_X86_64_IRELATIVE: u32 = 37;

    // ELF relocation types (aarch64)
    pub const R_AARCH64_COPY: u32 = 1024;

    // ELF relocation types (riscv64)
    pub const R_RISCV_COPY: u32 = 4;

    // ELF header
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct Ehdr {
        pub e_ident: [u8; 16],
        pub e_type: u16,
        pub e_machine: u16,
        pub e_version: u32,
        pub e_entry: u64,
        pub e_phoff: u64,
        pub e_shoff: u64,
        pub e_flags: u32,
        pub e_ehsize: u16,
        pub e_phentsize: u16,
        pub e_phnum: u16,
        pub e_shentsize: u16,
        pub e_shnum: u16,
        pub e_shstrndx: u16,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct Auxv {
        pub a_type: usize,
        pub a_un: AuxvUnion,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union AuxvUnion {
        pub a_val: usize,
        pub a_ptr: *const u8,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct Phdr {
        pub p_type: u32,
        pub p_flags: u32,
        pub p_offset: Off,
        pub p_vaddr: Addr,
        pub p_paddr: Addr,
        pub p_filesz: Xword,
        pub p_memsz: Xword,
        pub p_align: Xword,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct Dyn {
        pub d_tag: i64,
        pub d_un: DynUnion,
    }

    impl Ehdr {
        pub fn check_magic(&self) -> bool {
            self.e_ident[0] == 0x7f
                && self.e_ident[1] == b'E'
                && self.e_ident[2] == b'L'
                && self.e_ident[3] == b'F'
        }
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union DynUnion {
        pub d_val: usize,
        pub d_ptr: *const u8,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct Sym {
        pub st_name: u32,
        pub st_info: u8,
        pub st_other: u8,
        pub st_shndx: u16,
        pub st_value: Addr,
        pub st_size: Xword,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct Verdef {
        pub vd_version: u16,
        pub vd_flags: u16,
        pub vd_ndx: u16,
        pub vd_cnt: u16,
        pub vd_hash: u32,
        pub vd_aux: u32,
        pub vd_next: u32,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct Verneed {
        pub vn_version: u16,
        pub vn_cnt: u16,
        pub vn_file: u32,
        pub vn_aux: u32,
        pub vn_next: u32,
    }
}
