//! lock_monitor module.

pub mod registry;
#[cfg(feature = "host_examples")]
pub mod examples;
#[cfg(test)]
pub mod tests;

pub use self::registry::*;
use aop_macros::lock_monitor;
