//! procfs — Linux /proc virtual filesystem implementation.
//!
//! Provides /proc/self/maps, /proc/self/status, /proc/[pid]/stat,
//! /proc/meminfo, /proc/cpuinfo, /proc/uptime, /proc/version,
//! /proc/mounts, /proc/filesystems, /proc/loadavg, /proc/stat.

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use crate::interfaces::TaskId;
use crate::modules::vfs::{
    types::{DirEntry, File, FileStats},
    FileSystem,
};

pub mod generators;
mod vfs;


pub use generators::{
    generate_cmdline, generate_cpuinfo, generate_filesystems, generate_loadavg, generate_meminfo,
    generate_mounts, generate_self_maps, generate_self_stat, generate_self_status, generate_stat,
    generate_uptime, generate_version,
};

pub struct ProcFs;

impl ProcFs {
    pub fn new() -> Self {
        Self
    }

    #[allow(dead_code)]
    fn is_self_path(path: &str) -> bool {
        path == "self" || path.starts_with("self/")
    }

    #[allow(dead_code)]
    fn self_subpath(path: &str) -> Option<&str> {
        path.strip_prefix("self/")
    }
}

#[cfg(test)]
mod tests;
