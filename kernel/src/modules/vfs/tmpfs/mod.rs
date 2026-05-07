//! tmpfs — In-memory temporary filesystem for Linux compatibility.
//!
//! Provides a simple, fast, fully writable filesystem backed by kernel heap.
//! Used for /tmp, /run, /dev/shm mounting points.

extern crate alloc;

mod data;
mod node;
mod handle;
mod filesystem;
mod tests;

pub use filesystem::TmpFs;
