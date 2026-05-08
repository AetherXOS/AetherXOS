//! Memory service compatibility facade.
//!
//! The original repository splits memory responsibilities across several modules
//! (allocators, persistent_memory, memory_safety). Reexport available pieces so
//! callers can gradually migrate to `services::memory` without depending on a
//! single `modules::memory` module.

#[cfg(feature = "allocators")]
pub use crate::modules::allocators::*;

pub use crate::modules::persistent_memory::*;
pub use crate::modules::memory_safety::*;

