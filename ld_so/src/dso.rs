// ld_so/src/dso.rs
//! Dynamic Shared Object (DSO) Model.
//!
//! Represents a loaded ELF object (executable or library) in memory.
//! Handles parsing of Program Headers, Dynamic Section, and Symbol Tables.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{slice, str, mem};

use crate::{
    gnu_hash::GnuHash,
    header::elf,
    versioning::{VersionData, VersionReq},
};

pub struct DSO {
    pub name: String,
    pub base_addr: usize,
    pub phdrs: &'static [elf::Phdr],
    pub entry_point: usize,
    pub dynamic: Option<&'static [elf::Dyn]>,
    
    // Dynamic section values
    pub sym_table: Option<&'static [elf::Sym]>,
    pub str_table: Option<&'static [u8]>,
    pub gnu_hash: Option<GnuHash>,
    pub sysv_hash: Option<&'static [u32]>,
    
    pub rela_dyn: Option<&'static [elf::Rela]>,
    pub rela_plt: Option<&'static [elf::Rela]>,
    pub rela_count: usize,
    
    pub plt_got: Option<*const usize>,
    
    pub init: Option<unsafe extern "C" fn()>,
    pub init_array: Option<&'static [unsafe extern "C" fn()]>,
    pub fini: Option<unsafe extern "C" fn()>,
    pub fini_array: Option<&'static [unsafe extern "C" fn()]>,

    pub versym: Option<&'static [u16]>,
    pub verdef: Option<*const elf::Verdef>,
    pub verneed: Option<*const elf::Verneed>,
    pub verneed_num: usize,
    pub verdef_num: usize,

    // TLS data
    pub tls_module_id: usize,
    pub tls_offset: usize,
    pub tls_size: usize,
    pub tls_align: usize,
    pub tls_image: Option<&'static [u8]>,
}

impl DSO {
    pub unsafe fn new_executable(sp: *const usize) -> Self {
        let argc = *sp;
        let argv = sp.add(1);
        let mut envp = argv.add(argc + 1);
        while *envp != 0 {
            envp = envp.add(1);
        }
        envp = envp.add(1);

        let auxv = envp as *const elf::Auxv;

        let mut phdr_ptr = 0 as *const elf::Phdr;
        let mut phnum = 0;
        let mut entry = 0;

        let mut aux = auxv;
        while (*aux).a_type != elf::AT_NULL {
            match (*aux).a_type {
                elf::AT_PHDR => phdr_ptr = (*aux).a_un.a_ptr as *const elf::Phdr,
                elf::AT_PHNUM => phnum = (*aux).a_un.a_val,
                elf::AT_ENTRY => entry = (*aux).a_un.a_val,
                _ => (),
            }
            aux = aux.add(1);
        }

        let phdrs = slice::from_raw_parts(phdr_ptr, phnum);
        let (dynamic, base_addr) = Self::parse_phdrs_for_dynamic(phdrs);
        
        let mut dso = Self::empty();
        dso.name = String::from("main");
        dso.base_addr = base_addr;
        dso.phdrs = phdrs;
        dso.entry_point = entry;
        dso.dynamic = dynamic;
        dso.tls_module_id = 1;
        
        dso.parse_dynamic();
        dso.parse_phdrs_for_tls();

        dso
    }
    
    pub fn new_library(path: &str) -> Result<Self, ()> {
        // TODO: Proper file opening and mapping
        Err(())
    }

    fn empty() -> Self {
        Self {
            name: String::new(),
            base_addr: 0,
            phdrs: &[],
            entry_point: 0,
            dynamic: None,
            sym_table: None,
            str_table: None,
            gnu_hash: None,
            sysv_hash: None,
            rela_dyn: None,
            rela_plt: None,
            rela_count: 0,
            plt_got: None,
            init: None,
            init_array: None,
            fini: None,
            fini_array: None,
            versym: None,
            verdef: None,
            verneed: None,
            verneed_num: 0,
            verdef_num: 0,
            tls_module_id: 0,
            tls_offset: 0,
            tls_size: 0,
            tls_align: 0,
            tls_image: None,
        }
    }

    unsafe fn parse_phdrs_for_dynamic(phdrs: &[elf::Phdr]) -> (Option<&'static [elf::Dyn]>, usize) {
        let mut dynamic = None;
        let mut base_addr = 0;

        for phdr in phdrs {
            if phdr.p_type == elf::PT_LOAD && phdr.p_offset == 0 {
                base_addr = phdr.p_vaddr;
                break;
            }
        }

        for phdr in phdrs {
            if phdr.p_type == elf::PT_DYNAMIC {
                let mut dyn_ptr = (base_addr + phdr.p_vaddr) as *const elf::Dyn;
                let mut count = 0;
                while (*dyn_ptr).d_tag != elf::DT_NULL {
                    count += 1;
                    dyn_ptr = dyn_ptr.add(1);
                }
                dynamic = Some(slice::from_raw_parts(
                    (base_addr + phdr.p_vaddr) as *const elf::Dyn,
                    count,
                ));
                break;
            }
        }
        (dynamic, base_addr)
    }
    
    unsafe fn parse_phdrs_for_tls(&mut self) {
        for phdr in self.phdrs {
            if phdr.p_type == elf::PT_TLS {
                self.tls_image = Some(slice::from_raw_parts((self.base_addr + phdr.p_vaddr) as *const u8, phdr.p_filesz));
                self.tls_size = phdr.p_memsz;
                self.tls_align = phdr.p_align;
                break;
            }
        }
    }
    
    unsafe fn parse_dynamic(&mut self) {
        let dynamic = match self.dynamic {
            Some(d) => d,
            None => return,
        };
        
        let get_val = |tag| dynamic.iter().find(|d| d.d_tag == tag).map(|d| self.base_addr + d.d_un.d_ptr);

        self.sym_table = get_val(elf::DT_SYMTAB).map(|p| slice::from_raw_parts(p as *const elf::Sym, 0)); // Size is unknown here
        self.str_table = get_val(elf::DT_STRTAB).map(|p| slice::from_raw_parts(p as *const u8, 0)); // Size is unknown here
        
        if let Some(str_sz) = get_val(elf::DT_STRSZ) {
            self.str_table = self.str_table.map(|s| slice::from_raw_parts(s.as_ptr(), str_sz));
        }
        if let Some(sym_ent) = get_val(elf::DT_SYMENT) {
            // This is tricky. We need to find the end of the symbol table.
            // A common way is to use the string table size, but that's not guaranteed.
            // Let's assume for now that the .dynsym section is contiguous.
            // A better approach would be to use section headers if available.
        }

        self.rela_dyn = get_val(elf::DT_RELA).map(|p| slice::from_raw_parts(p as *const elf::Rela, get_val(elf::DT_RELASZ).unwrap_or(0) / mem::size_of::<elf::Rela>()));
        self.rela_plt = get_val(elf::DT_JMPREL).map(|p| slice::from_raw_parts(p as *const elf::Rela, get_val(elf::DT_PLTRELSZ).unwrap_or(0) / mem::size_of::<elf::Rela>()));
        self.rela_count = get_val(elf::DT_RELACOUNT).unwrap_or(0);
        
        self.init = get_val(elf::DT_INIT).map(|p| mem::transmute(p));
        self.init_array = get_val(elf::DT_INIT_ARRAY).map(|p| slice::from_raw_parts(p as *const unsafe extern "C" fn(), get_val(elf::DT_INIT_ARRAYSZ).unwrap_or(0) / mem::size_of::<usize>()));
        self.fini = get_val(elf::DT_FINI).map(|p| mem::transmute(p));
        self.fini_array = get_val(elf::DT_FINI_ARRAY).map(|p| slice::from_raw_parts(p as *const unsafe extern "C" fn(), get_val(elf::DT_FINI_ARRAYSZ).unwrap_or(0) / mem::size_of::<usize>()));
    }

    pub unsafe fn run_init(&self) {
        if let Some(init_func) = self.init {
            init_func();
        }
        if let Some(init_arr) = self.init_array {
            for func in init_arr {
                func();
            }
        }
    }
    
    pub fn relro_segment(&self) -> Option<&'static elf::Phdr> {
        self.phdrs.iter().find(|phdr| phdr.p_type == elf::PT_GNU_RELRO)
    }

    pub fn relocations(&self) -> impl Iterator<Item = (u32, usize, usize, Option<usize>)> {
        let rela_dyn_iter = self.rela_dyn.unwrap_or(&[]).iter();
        let rela_plt_iter = self.rela_plt.unwrap_or(&[]).iter();

        rela_dyn_iter.chain(rela_plt_iter).map(|rela| {
            let r_type = (rela.r_info & 0xFFFFFFFF) as u32;
            let sym_idx = (rela.r_info >> 32) as usize;
            (r_type, sym_idx, rela.r_offset, Some(rela.r_addend))
        })
    }

    pub fn get_sym_name(&self, index: usize) -> Option<&str> {
        let sym = &self.sym_table?[index];
        if sym.st_name == 0 {
            return None;
        }
        let start = sym.st_name as usize;
        let slice = &self.str_table?[start..];
        let end = slice.iter().position(|&c| c == 0)?;
        str::from_utf8(&slice[..end]).ok()
    }

    pub fn get_version_req(&self, _sym_idx: usize) -> Option<VersionReq> {
        None
    }

    pub fn sym_table(&self) -> &[elf::Sym] {
        self.sym_table.unwrap_or(&[])
    }
    pub fn str_table(&self) -> &[u8] {
        self.str_table.unwrap_or(&[])
    }
    pub fn gnu_hash(&self) -> Option<&GnuHash> {
        self.gnu_hash.as_ref()
    }
    pub fn sysv_hash(&self) -> Option<&[u32]> {
        self.sysv_hash
    }
    pub fn base_addr(&self) -> usize {
        self.base_addr
    }

    pub fn version_data(&self) -> Option<VersionData<'static>> {
        if let (Some(versym), Some(str_tab)) = (self.versym, self.str_table) {
            Some(VersionData {
                versym,
                verneed: self.verneed.unwrap_or(core::ptr::null()),
                verneed_num: self.verneed_num,
                verdef: self.verdef.unwrap_or(core::ptr::null()),
                verdef_num: self.verdef_num,
                str_tab,
            })
        } else {
            None
        }
    }
}
