//! Lightweight prelude for common shared types.

pub use crate::identifiers::TypedId;
pub use crate::result::{SharedError, SharedResult};
pub use crate::telemetry::{key as telemetry_key, suffix as telemetry_suffix};
pub use crate::target_arch::{ParseTargetArchError, TargetArch};
pub use crate::units::{gib, kib, mib, ms_to_ns, sec_to_ms, PAGE_SIZE_4K};
