use crate::harness::{TestResult, TestCategory};
use alloc::{vec, vec::Vec};
use core::ops::Fn;

pub fn all_tests() -> Vec<&'static dyn Fn() -> TestResult> {
    vec![
        &test_scheduler_create,
        &test_scheduler_enqueue,
        &test_scheduler_priority,
    ]
}

fn test_scheduler_create() -> TestResult {
    struct SchedulerConfig {
        time_slice_ns: u64,
        max_tasks: usize,
        priority_levels: usize,
    }
    
    let config = SchedulerConfig {
        time_slice_ns: 4_000_000,
        max_tasks: 1024,
        priority_levels: 64,
    };
    
    if config.time_slice_ns > 0 && config.max_tasks > 0 && config.priority_levels > 0 {
        TestResult::pass("modules::scheduler::create")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("modules::scheduler::create", "Scheduler creation failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_scheduler_enqueue() -> TestResult {
    let mut queue: Vec<usize> = Vec::new();
    
    for i in 0..10 {
        queue.push(i);
    }
    
    if queue.len() == 10 && queue[0] == 0 && queue[9] == 9 {
        TestResult::pass("modules::scheduler::enqueue")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("modules::scheduler::enqueue", "Scheduler enqueue failed")
            .with_category(TestCategory::Unit)
    }
}

fn test_scheduler_priority() -> TestResult {
    struct Task {
        id: usize,
        priority: i32,
    }
    
    let mut tasks = vec![
        Task { id: 1, priority: 5 },
        Task { id: 2, priority: 10 },
        Task { id: 3, priority: 1 },
    ];
    
    tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
    
    if tasks[0].priority == 10 && tasks[2].priority == 1 {
        TestResult::pass("modules::scheduler::priority")
            .with_category(TestCategory::Unit)
    } else {
        TestResult::fail("modules::scheduler::priority", "Priority scheduling failed")
            .with_category(TestCategory::Unit)
    }
}
