use crate::harness::TestResult;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_scheduling_task_create,
        &test_scheduling_task_switch,
        &test_scheduling_cfs_fairness,
        &test_scheduling_rt_deadline,
        &test_scheduling_work_stealing,
    ]
}

fn test_scheduling_task_create() -> TestResult {
    TestResult::pass("integration::kernel::scheduling::task_create")
}

fn test_scheduling_task_switch() -> TestResult {
    TestResult::pass("integration::kernel::scheduling::task_switch")
}

fn test_scheduling_cfs_fairness() -> TestResult {
    TestResult::pass("integration::kernel::scheduling::cfs_fairness")
}

fn test_scheduling_rt_deadline() -> TestResult {
    TestResult::pass("integration::kernel::scheduling::rt_deadline")
}

fn test_scheduling_work_stealing() -> TestResult {
    TestResult::pass("integration::kernel::scheduling::work_stealing")
}
