//! elf_dynamic module.

use crate::{klog_debug, klog_warn};

/// x86_64 Relocation types (partial, for demonstration)
#[repr(u32)]
#[allow(non_camel_case_types)]
pub enum RelocTypeX86_64 {
    NONE = 0,
    _64 = 1,
    PC32 = 2,
    GOT32 = 3,
    PLT32 = 4,
    COPY = 5,
    GLOB_DAT = 6,
    JMP_SLOT = 7,
    RELATIVE = 8,
    GOTPCREL = 9,
    GOTPCRELX = 41,
    _32 = 10,
    _32S = 11,
    DTPMOD64 = 16,
    DTPOFF64 = 17,
    TPOFF64 = 18,
    TLSGD = 19,
    TLSLD = 20,
    DTPOFF32 = 21,
    GOTTPOFF = 22,
    TPOFF32 = 23,
    IRELATIVE = 37,
}

mod relocate;
pub use relocate::{process_relocations, process_relocations_inplace};
mod reader;
pub use reader::{parse_sysv_hash_nchain, parse_gnu_hash_symbol_count};
mod access;
pub use access::{ptr_copy_bytes_from_slice, ptr_read_unaligned_u32, ptr_read_unaligned_u64, ptr_write_unaligned_u64};

/// ELF .dynamic section and relocation parsing for dynamic linker

mod parser;
pub use parser::{DynamicEntry, DynamicSection, RelocationEntry};

// Hash parsing helpers moved to `reader.rs` for modularity.

pub enum RelocationType {
    Rel,
    Rela,
}

pub struct RelocationTable {
    pub entries: alloc::vec::Vec<RelocationEntry>,
    pub rel_type: RelocationType,
}

impl RelocationTable {
    // Parsing implementation moved to `relocate.rs` for separation of concerns.
}

#[cfg(all(test, any()))]
mod tests;


