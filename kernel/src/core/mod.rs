//! Architecture-neutral core contracts.
//!
//! This layer is the stable seam for capability-oriented code. For now it bridges the
//! existing interface modules so the codebase can migrate without losing any scheduler,
//! driver, or hardware surface.

pub mod error;
pub mod log;
pub mod log_filter;
pub mod time;
pub mod traits;
pub mod types;

pub use error::{KernelError, KernelResult};