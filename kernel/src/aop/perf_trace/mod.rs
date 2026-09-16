//! perf_trace module.

pub mod registry;
#[cfg(feature = "host_examples")]
pub mod examples;
#[cfg(test)]
pub mod tests;

pub use self::registry::*;
use aop_macros::perf_trace;
