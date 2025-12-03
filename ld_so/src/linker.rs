// ld_so/src/linker.rs
//! The Dynamic Linker Orchestrator.
//! Manages the loading graph, symbol resolution, relocation, and TLS initialization.

use alloc::{
    collections::{BTreeMap, VecDeque},
    string::{String, ToString},
    vec::Vec,
};
use core::{mem, ptr, slice, str};

use crate::{
    dso::DSO,
    header::elf,
    linux_parity::{LookupResult, find_symbol_linux_style},
    reloc,
    tcb::Tcb,
    tls,
    versioning::{VersionData, VersionReq},
};

const DEFAULT_STATIC_TLS_SURPLUS: usize = 2048;

unsafe extern "C" {
    fn open(path: *const i8, flags: i32, mode: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fstat(fd: i32, buf: *mut u8) -> i32;
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut u8;
    fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
}

pub struct Linker {
    objects: Vec<DSO>,
    loaded_names: Vec<String>,
    global_symbols: BTreeMap<String, (LookupResult, usize, usize)>,
    static_tls_size: usize,
    static_tls_end_offset: usize,
    static_tls_align: usize,
    tls_offset: usize,
    surplus_remaining: usize,
    surplus_size: usize,
}

impl Linker {
    pub fn new(envp: *const *const i8) -> Self {
        let surplus_size = Self::parse_tunables(envp).unwrap_or(DEFAULT_STATIC_TLS_SURPLUS);

        Self {
            objects: Vec::new(),
            loaded_names: Vec::new(),
            global_symbols: BTreeMap::new(),
            static_tls_size: 0,
            static_tls_end_offset: 0,
            static_tls_align: 16,
            tls_offset: 0,
            surplus_remaining: 0,
            surplus_size,
        }
    }

    fn parse_tunables(envp: *const *const i8) -> Option<usize> {
        if envp.is_null() { return None; }
        unsafe {
            let mut i = 0;
            loop {
                let entry_ptr = *envp.add(i);
                if entry_ptr.is_null() { break; }
                if str::from_utf8_unchecked(slice::from_raw_parts(entry_ptr as *const u8, 15)) == "GLIBC_TUNABLES=" {
                    return Self::parse_surplus_from_tunable_string(entry_ptr.add(15));
                }
                i += 1;
            }
        }
        None
    }

    unsafe fn parse_surplus_from_tunable_string(ptr: *const i8) -> Option<usize> {
        let target = b"glibc.rtld.optional_static_tls=";
        let mut cursor = ptr;
        loop {
            if str::from_utf8_unchecked(slice::from_raw_parts(cursor as *const u8, target.len())) == target {
                let mut size = 0;
                let mut p = cursor.add(target.len());
                while *p >= b'0' as i8 && *p <= b'9' as i8 {
                    size = size * 10 + (*p as u8 - b'0') as usize;
                    p = p.add(1);
                }
                return Some(size);
            }
            while *cursor != 0 && *cursor != b':' as i8 { cursor = cursor.add(1); }
            if *cursor == 0 { break; }
            cursor = cursor.add(1);
        }
        None
    }

    pub fn link(&mut self, main_dso: DSO) {
        self.add_object(main_dso);
        self.load_dependencies();
        self.layout_static_tls();
        self.build_global_sym_map();

        unsafe {
            let tcb = Tcb::new(self.static_tls_size, self.static_tls_align);
            if !tcb.is_null() {
                (*tcb).activate();
                self.initialize_static_tls(tcb);
            }
        }

        for i in 0..self.objects.len() { self.relocate_single(i); }
        self.finalize_relro();
        for i in (0..self.objects.len()).rev() { unsafe { self.objects[i].run_init(); } }
    }

    fn add_object(&mut self, dso: DSO) {
        self.loaded_names.push(dso.name.clone());
        self.objects.push(dso);
    }

    pub fn get_entry_point(&self) -> usize {
        self.objects.first().map_or(0, |dso| dso.entry_point)
    }

    fn load_dependencies(&mut self) {
        let mut queue: VecDeque<usize> = (0..self.objects.len()).collect();
        while let Some(dso_idx) = queue.pop_front() {
            let dso = &self.objects[dso_idx];
            if dso.dynamic.is_none() { continue; }
            let dynamic = dso.dynamic.unwrap();
            let str_table = dso.str_table.unwrap_or(&[]);

            for dyn_entry in dynamic {
                if dyn_entry.d_tag == elf::DT_NEEDED {
                    let name_offset = dyn_entry.d_un.d_val;
                    let name_bytes = &str_table[name_offset..];
                    let name_end = name_bytes.iter().position(|&c| c == 0).unwrap_or(name_bytes.len());
                    let name = unsafe { str::from_utf8_unchecked(&name_bytes[..name_end]) };

                    if !self.loaded_names.iter().any(|n| n == name) {
                        let path = format!("/lib/{}", name);
                        if let Ok(new_dso) = DSO::new_library(&path) {
                            let new_idx = self.objects.len();
                            self.add_object(new_dso);
                            queue.push_back(new_idx);
                        }
                    }
                }
            }
        }
    }
    
    fn build_global_sym_map(&mut self) {
        for (i, dso) in self.objects.iter().enumerate() {
            for (sym_idx, sym) in dso.sym_table().iter().enumerate() {
                if sym.st_name != 0 && (sym.st_info & 0xF) != elf::STT_FILE && sym.st_shndx != elf::SHN_UNDEF {
                    if let Some(name) = dso.get_sym_name(sym_idx) {
                        if !self.global_symbols.contains_key(name) {
                             let res = LookupResult { value: dso.base_addr + sym.st_value, size: sym.st_size };
                             self.global_symbols.insert(name.to_string(), (res, dso.tls_module_id, dso.tls_offset));
                        }
                    }
                }
            }
        }
    }

    fn layout_static_tls(&mut self) {
        self.tls_offset = 0;
        self.static_tls_align = 16;
        for (i, obj) in self.objects.iter_mut().enumerate() {
            if obj.tls_size == 0 { continue; }
            obj.tls_module_id = i + 1;
            let align_mask = obj.tls_align - 1;
            self.tls_offset = (self.tls_offset + align_mask) & !align_mask;
            obj.tls_offset = self.tls_offset;
            self.tls_offset += obj.tls_size;
            if obj.tls_align > self.static_tls_align { self.static_tls_align = obj.tls_align; }
        }
        self.static_tls_end_offset = self.tls_offset;
        self.surplus_remaining = self.surplus_size;
        self.static_tls_size = self.static_tls_end_offset + self.surplus_size;
    }

    pub fn try_allocate_static_tls(&mut self, size: usize, align: usize) -> Option<usize> {
        let current_end = self.static_tls_size - self.surplus_remaining;
        let align_mask = align - 1;
        let start = (current_end + align_mask) & !align_mask;
        let end = start + size;
        if end <= self.static_tls_size {
            self.surplus_remaining = self.static_tls_size - end;
            Some(start)
        } else {
            None
        }
    }

    unsafe fn initialize_static_tls(&self, tcb: *mut Tcb) {
        let tcb_addr = tcb as usize;
        #[cfg(target_arch = "x86_64")]
        let block_start = tcb_addr.wrapping_sub(self.static_tls_size);
        #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
        let block_start = {
            let tcb_size = mem::size_of::<Tcb>();
            let tcb_aligned = (tcb_size + self.static_tls_align - 1) & !(self.static_tls_align - 1);
            tcb_addr + tcb_aligned
        };
        for obj in &self.objects {
            if obj.tls_size == 0 { continue; }
            let dest_addr = block_start + obj.tls_offset;
            let dest = dest_addr as *mut u8;
            if let Some(image) = obj.tls_image { ptr::copy_nonoverlapping(image.as_ptr(), dest, image.len()); }
            let image_len = obj.tls_image.map(|s| s.len()).unwrap_or(0);
            if obj.tls_size > image_len {
                ptr::write_bytes(dest.add(image_len), 0, obj.tls_size - image_len);
            }
        }
    }
    
    fn finalize_relro(&self) {
        for dso in &self.objects {
            if let Some(relro) = dso.relro_segment() {
                unsafe { mprotect((dso.base_addr + relro.p_vaddr) as *mut u8, relro.p_memsz, elf::PROT_READ); }
            }
        }
    }

    fn relocate_single(&self, obj_idx: usize) {
        let obj = &self.objects[obj_idx];
        for (r_type, sym_idx, offset, addend) in obj.relocations() {
            let reloc_addr = obj.base_addr + offset;
            if unsafe { reloc::relocate(r_type, 0, 0, reloc_addr, addend, obj.base_addr, obj.tls_module_id, obj.tls_offset, self.static_tls_size) } {
                continue;
            }
            let sym_name = match obj.get_sym_name(sym_idx) {
                Some(s) => s,
                None => continue,
            };
            if let Some((res, tls_id, tls_off)) = self.lookup_symbol(sym_name) {
                unsafe {
                    if !reloc::relocate(r_type, res.value, res.size, reloc_addr, addend, obj.base_addr, *tls_id, *tls_off, self.static_tls_size) {
                        reloc::relocate_copy(r_type, res.value, reloc_addr, res.size);
                    }
                }
            }
        }
    }

    fn lookup_symbol(&self, name: &str) -> Option<&(LookupResult, usize, usize)> {
        self.global_symbols.get(name)
    }
    
    pub fn dlopen(&mut self, path: &str) -> Option<usize> {
        if let Some(idx) = self.loaded_names.iter().position(|n| n == path) { return Some(idx + 1); }
        let new_dso = DSO::new_library(path).ok()?;
        let start_idx = self.objects.len();
        self.add_object(new_dso);
        self.load_dependencies();
        
        for i in start_idx..self.objects.len() {
            let obj = &mut self.objects[i];
            if obj.tls_size > 0 {
                if let Some(offset) = self.try_allocate_static_tls(obj.tls_size, obj.tls_align) {
                    obj.tls_offset = offset;
                } else {
                    obj.tls_module_id = tls::register_tls_module(obj.tls_size, obj.tls_align, obj.tls_image.map(|s| s.as_ptr() as usize).unwrap_or(0), obj.tls_image.map(|s| s.len()).unwrap_or(0), None);
                }
            }
        }
        
        for i in start_idx..self.objects.len() { self.relocate_single(i); }
        for i in start_idx..self.objects.len() {
            let dso = &self.objects[i];
            if let Some(relro) = dso.relro_segment() {
                unsafe { mprotect((dso.base_addr + relro.p_vaddr) as *mut u8, relro.p_memsz, elf::PROT_READ); }
            }
        }
        for i in (start_idx..self.objects.len()).rev() { unsafe { self.objects[i].run_init(); } }
        
        Some(start_idx + 1)
    }
    
    pub fn dlsym(&self, handle: usize, symbol: &str) -> Option<usize> {
        if handle == 0 { // RTLD_DEFAULT
            return self.lookup_symbol(symbol).map(|(res, _, _)| res.value);
        }
        let dso_idx = handle - 1;
        if dso_idx >= self.objects.len() { return None; }
        
        // This is a simplification. A real implementation would do a dependency graph search.
        self.lookup_symbol(symbol).map(|(res, _, _)| res.value)
    }
}
