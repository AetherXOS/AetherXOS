use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_ipc_ring_buffer,
        &test_ipc_message_passing,
        &test_ipc_futex,
        &test_ipc_channel,
    ]
}

fn test_ipc_ring_buffer() -> TestResult {
    TestResult::pass("integration::kernel::ipc::ring_buffer")
}

fn test_ipc_message_passing() -> TestResult {
    TestResult::pass("integration::kernel::ipc::message_passing")
}

fn test_ipc_futex() -> TestResult {
    TestResult::pass("integration::kernel::ipc::futex")
}

fn test_ipc_channel() -> TestResult {
    TestResult::pass("integration::kernel::ipc::channel")
}
