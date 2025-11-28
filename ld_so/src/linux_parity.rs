// Minimal Linux parity stubs for ld_so
// These provide the necessary symbols used by the linker.

use crate::header::elf;
use core::ptr;

/// Result of a symbol lookup.
#[derive(Debug, Clone, Copy)]
pub struct LookupResult {
    pub value: usize,
    pub size: usize,
}

/// Find a symbol using Linux-style lookup (GNU hash, etc.).
/// This is a placeholder that always returns None.
pub fn find_symbol_linux_style(
    _name: &str,
    _ver_req: Option<&crate::versioning::VersionReq>,
    _symtab: &[elf::Sym],
    _strtab: &[u8],
    _gnu_hash: Option<&crate::gnu_hash::GnuHash>, // placeholder type
    _sysv_hash: Option<&[u32]>,
    _versym: Option<&[u16]>,
    _base_addr: usize,
) -> Option<LookupResult> {
    // Real implementation would search the symbol tables.
    None
}

/// Resolve an IFUNC symbol. Placeholder returns the same address.
pub fn resolve_ifunc(_addr: usize) -> usize {
    _addr
}
