pub mod registry;
pub mod examples;
#[cfg(test)]
pub mod tests;

pub use self::registry::*;
use aop_macros::perf_trace;
