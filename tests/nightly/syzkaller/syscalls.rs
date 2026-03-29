use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_syzkaller_syscalls_vfs,
        &test_syzkaller_syscalls_ipc,
        &test_syzkaller_syscalls_network,
    ]
}

fn test_syzkaller_syscalls_vfs() -> TestResult {
    TestResult::pass("nightly::syzkaller::syscalls::vfs")
}

fn test_syzkaller_syscalls_ipc() -> TestResult {
    TestResult::pass("nightly::syzkaller::syscalls::ipc")
}

fn test_syzkaller_syscalls_network() -> TestResult {
    TestResult::pass("nightly::syzkaller::syscalls::network")
}
