use super::*;

#[path = "tests/numeric/mod.rs"]
mod numeric_tests;

#[cfg(all(test, feature = "vfs", feature = "posix_fs", feature = "posix_time"))]
#[path = "tests/fs/mod.rs"]
mod fs_tests;
