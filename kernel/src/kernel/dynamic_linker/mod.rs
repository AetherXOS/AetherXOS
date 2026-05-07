//! Dynamic Linker (ld.so) infrastructure for UNIX/POSIX/Linux compatibility
//!
//! This module provides the core logic for dynamic linking, relocation, and symbol resolution
//! for ELF binaries and shared objects. It is invoked when a PT_INTERP (interpreter) is present
//! in the ELF executable, and is responsible for loading DT_NEEDED dependencies, applying relocations,
//! and resolving symbols at runtime.

use xmas_elf::program::Type;
use xmas_elf::ElfFile;

pub mod api;
pub mod elf_dynamic;
mod entry;
mod helpers;
pub mod so_loader;
pub mod symbol;

pub use self::entry::dynamic_linker_entry;
