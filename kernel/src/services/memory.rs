//! Memory service facade.
//!
//! Re-exports allocator types so callers can use `services::memory`
//! without depending directly on `modules::allocators`.

#[cfg(feature = "allocators")]
pub use crate::modules::allocators::*;

