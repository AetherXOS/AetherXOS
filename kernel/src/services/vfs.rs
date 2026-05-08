//! VFS service compatibility facade.
//!
//! Reexport the existing vfs module surface so code can reference services::vfs
//! during migration without changing behavior.

#[cfg(feature = "vfs")]
pub use crate::modules::vfs::*;

