#![cfg_attr(not(feature = "clap"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(feature = "clap")]
extern crate std;

pub mod identifiers;
pub mod macros;
pub mod prelude;
pub mod result;
pub mod target_arch;
pub mod telemetry;
pub mod units;

pub use target_arch::TargetArch;
