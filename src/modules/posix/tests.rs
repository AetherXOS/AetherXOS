#[path = "tests/numeric_tests.rs"]
mod numeric_tests;

#[cfg(all(test, feature = "vfs", feature = "posix_fs", feature = "posix_time"))]
#[path = "tests/fs_tests.rs"]
mod fs_tests;

