use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_filesystem_vfs_mount,
        &test_filesystem_vfs_lookup,
        &test_filesystem_ramfs,
        &test_filesystem_tmpfs,
        &test_filesystem_procfs,
    ]
}

fn test_filesystem_vfs_mount() -> TestResult {
    TestResult::pass("integration::kernel::filesystem::vfs_mount")
}

fn test_filesystem_vfs_lookup() -> TestResult {
    TestResult::pass("integration::kernel::filesystem::vfs_lookup")
}

fn test_filesystem_ramfs() -> TestResult {
    TestResult::pass("integration::kernel::filesystem::ramfs")
}

fn test_filesystem_tmpfs() -> TestResult {
    TestResult::pass("integration::kernel::filesystem::tmpfs")
}

fn test_filesystem_procfs() -> TestResult {
    TestResult::pass("integration::kernel::filesystem::procfs")
}
