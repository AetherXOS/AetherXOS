//! Service layer bridge.
//!
//! This namespace will eventually host scheduler, VFS, memory, and driver services.
//! For now it reexports the existing module families so nothing is removed during the
//! architecture migration.

pub mod scheduler;
pub mod vfs;
pub mod memory;
pub mod drivers;