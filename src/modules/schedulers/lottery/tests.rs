use super::{
    deterministic_replay_trace, latest_replay_event, lottery_runtime_config,
    set_lottery_runtime_config, Lottery, LotteryReplayTask, LotteryRuntimeConfig,
};
use crate::interfaces::{KernelTask, Scheduler, TaskId};

fn make_task(id: usize, priority: u8) -> KernelTask {
    KernelTask::new(TaskId(id), priority, 0, 0, 0, 0, 0)
}

#[test_case]
fn lottery_ticket_weight_matches_global_priority_semantics() {
    let high = Lottery::ticket_count_for_priority(0);
    let medium = Lottery::ticket_count_for_priority(128);
    let low = Lottery::ticket_count_for_priority(255);
    assert!(high > medium);
    assert!(medium > low);
}

#[test_case]
fn lottery_higher_priority_task_wins_more_often_with_fixed_seed() {
    let mut sched = Lottery::new();
    sched.add_task(make_task(1, 0));
    sched.add_task(make_task(2, 255));

    let mut hi = 0usize;
    let mut lo = 0usize;
    for _ in 0..2048 {
        match sched.pick_next() {
            Some(TaskId(1)) => hi += 1,
            Some(TaskId(2)) => lo += 1,
            _ => {}
        }
    }
    assert!(hi > lo);
}

#[test_case]
fn lottery_replay_trace_is_deterministic_for_same_seed() {
    let tasks = [
        LotteryReplayTask::new(TaskId(1), 0),
        LotteryReplayTask::new(TaskId(2), 128),
        LotteryReplayTask::new(TaskId(3), 255),
    ];
    let a = deterministic_replay_trace(0xCAFEBABE, &tasks, 24);
    let b = deterministic_replay_trace(0xCAFEBABE, &tasks, 24);
    assert_eq!(a, b);
}

#[test_case]
fn lottery_replay_trace_changes_when_seed_changes() {
    let tasks = [
        LotteryReplayTask::new(TaskId(1), 0),
        LotteryReplayTask::new(TaskId(2), 128),
        LotteryReplayTask::new(TaskId(3), 255),
    ];
    let a = deterministic_replay_trace(0xCAFEBABE, &tasks, 24);
    let b = deterministic_replay_trace(0xCAFEBABF, &tasks, 24);
    assert_ne!(a, b);
}

#[test_case]
fn lottery_replay_known_prefix_stays_stable() {
    let tasks = [
        LotteryReplayTask::new(TaskId(1), 0),
        LotteryReplayTask::new(TaskId(2), 128),
        LotteryReplayTask::new(TaskId(3), 255),
    ];
    let trace = deterministic_replay_trace(0xCAFEBABE, &tasks, 12);
    let expected = [
        Some(TaskId(2)),
        Some(TaskId(1)),
        Some(TaskId(1)),
        Some(TaskId(1)),
        Some(TaskId(1)),
        Some(TaskId(1)),
        Some(TaskId(1)),
        Some(TaskId(1)),
        Some(TaskId(1)),
        Some(TaskId(1)),
        Some(TaskId(2)),
        Some(TaskId(1)),
    ];
    assert_eq!(trace.as_slice(), &expected);
}

#[test_case]
fn lottery_runtime_replay_ring_records_latest_pick() {
    let mut sched = Lottery::with_seed(0xCAFEBABE);
    sched.add_task(make_task(11, 0));
    sched.add_task(make_task(22, 255));
    let picked = sched.pick_next();
    let latest = latest_replay_event();
    assert!(picked.is_some());
    assert!(latest.is_some());
    if let (Some(p), Some(ev)) = (picked, latest) {
        assert_eq!(ev.task_id, p);
        assert!(ev.total_tickets > 0);
    }
}

#[test_case]
fn lottery_runtime_config_roundtrip() {
    let original = lottery_runtime_config();
    let updated = LotteryRuntimeConfig {
        initial_seed: 0x1234_5678,
        tickets_per_priority_level: 21,
        min_tickets_per_task: 3,
        lcg_multiplier: 17,
        lcg_increment: 5,
    };
    set_lottery_runtime_config(updated);
    let after = lottery_runtime_config();
    assert_eq!(after.initial_seed, updated.initial_seed);
    assert_eq!(
        after.tickets_per_priority_level,
        updated.tickets_per_priority_level
    );
    assert_eq!(after.min_tickets_per_task, updated.min_tickets_per_task);
    assert_eq!(after.lcg_multiplier, updated.lcg_multiplier);
    assert_eq!(after.lcg_increment, updated.lcg_increment);
    set_lottery_runtime_config(original);
}

#[test_case]
fn lottery_lcg_params_are_normalized_to_non_zero_odd_values() {
    let original = lottery_runtime_config();
    set_lottery_runtime_config(LotteryRuntimeConfig {
        initial_seed: original.initial_seed,
        tickets_per_priority_level: original.tickets_per_priority_level,
        min_tickets_per_task: original.min_tickets_per_task,
        lcg_multiplier: 8,
        lcg_increment: 0,
    });
    let after = lottery_runtime_config();
    assert_eq!(after.lcg_multiplier % 2, 1);
    assert!(after.lcg_multiplier >= 1);
    assert_eq!(after.lcg_increment % 2, 1);
    assert!(after.lcg_increment >= 1);
    set_lottery_runtime_config(original);
}
