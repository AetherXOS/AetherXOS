//! Process launch subsystem: bootstrapping, spawning, lifecycle handoff, and observability.

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::interfaces::task::{ProcessId, TaskId};
use crate::interfaces::Scheduler;

#[cfg(feature = "process_abstraction")]
use crate::kernel::process::Process;

pub mod state;
pub mod types;
pub mod stats;

pub use state::*;
pub use types::*;
pub use stats::*;

#[cfg(feature = "process_abstraction")]
pub mod process_runtime;

#[cfg(feature = "process_abstraction")]
pub use process_runtime::*;

#[cfg(test)]
mod tests;

// `stats()` moved to `stats.rs` for modularization.

