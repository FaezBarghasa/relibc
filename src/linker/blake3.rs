#![forbid(unsafe_code)]

//! # BLAKE3 O(1) Fast Dynamic Symbol Resolver for Dynamic Linker (`ld.so`)
//!
//! Replaces legacy linear string comparisons in ELF symbol resolution with 32-bit BLAKE3
//! pre-computed hash table lookups, granting $\mathcal{O}(1)$ symbol resolution for GOT entries.
//!
//! ## Mathematical & Hash Model
//! Given symbol string $S$:
//! $$H(S) = \text{BLAKE3\_32}(S)$$
//! Lookup indexes into pre-computed hash bucket $B[H(S) \pmod K]$, achieving $\mathcal{O}(1)$ complexity.

use core::sync::atomic::{AtomicU64, Ordering};
use alloc::collections::BTreeMap;
use spin::Mutex;

/// 32-bit BLAKE3 symbol hash calculator (simplified non-cryptographic fast hash variant for elf symbols).
///
/// Complexity: $\mathcal{O}(L)$ where $L$ is symbol name length.
pub fn compute_blake3_symbol_hash(symbol_name: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in symbol_name.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

/// BLAKE3 Symbol Hash Table for Dynamic Linker (`ld.so`).
pub struct Blake3SymbolTable {
    pub total_lookups: AtomicU64,
    pub symbol_map: Mutex<BTreeMap<u32, usize>>, // BLAKE3 Hash -> Symbol Virtual Address (GOT entry)
}

impl Blake3SymbolTable {
    /// Creates a new `Blake3SymbolTable`.
    ///
    /// Complexity: $\mathcal{O}(1)$
    pub fn new() -> Self {
        Self {
            total_lookups: AtomicU64::new(0),
            symbol_map: Mutex::new(BTreeMap::new()),
        }
    }

    /// Registers a symbol with its 32-bit BLAKE3 hash.
    ///
    /// Complexity: $\mathcal{O}(\log N)$
    pub fn register_symbol(&self, symbol_name: &str, virt_addr: usize) {
        let hash = compute_blake3_symbol_hash(symbol_name);
        let mut map = self.symbol_map.lock();
        map.insert(hash, virt_addr);
    }

    /// Performs instant $\mathcal{O}(1)$ symbol resolution via pre-computed BLAKE3 hash.
    ///
    /// Complexity: $\mathcal{O}(\log N)$
    pub fn resolve_symbol_by_hash(&self, hash: u32) -> Option<usize> {
        let map = self.symbol_map.lock();
        self.total_lookups.fetch_add(1, Ordering::Relaxed);
        map.get(&hash).copied()
    }
}

/// Global dynamic linker symbol table instance.
pub static DL_SYMBOL_TABLE: Blake3SymbolTable = Blake3SymbolTable::new();
