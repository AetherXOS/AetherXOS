//! Dynamic Linker (ld.so) infrastructure for UNIX/POSIX/Linux compatibility
//!
//! This module provides the core logic for dynamic linking, relocation, and symbol resolution
//! for ELF binaries and shared objects. It is invoked when a PT_INTERP (interpreter) is present
//! in the ELF executable, and is responsible for loading DT_NEEDED dependencies, applying relocations,
//! and resolving symbols at runtime.

use xmas_elf::program::Type;
use xmas_elf::ElfFile;

#[path = "api.rs"]
pub mod api;
#[path = "elf_dynamic.rs"]
pub mod elf_dynamic;
#[path = "dynamic_linker_entry.rs"]
mod entry;
#[path = "dynamic_linker_helpers.rs"]
mod helpers;
#[path = "so_loader/mod.rs"]
pub mod so_loader;
#[path = "symbol.rs"]
pub mod symbol;

pub use self::entry::dynamic_linker_entry;
