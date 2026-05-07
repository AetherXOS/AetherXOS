/// Per-task scheduling statistics (like Linux schedstat).
#[derive(Debug, Clone, Copy, Default)]
pub struct SchedStat {
    /// Total CPU time consumed (ns).
    pub cpu_time_ns: u64,
    /// Total time spent waiting in runqueue (ns).
    pub wait_time_ns: u64,
    /// Number of times this task was scheduled.
    pub run_count: u64,
    /// Number of involuntary preemptions.
    pub preempt_count: u64,
    /// Timestamp when task was last enqueued (for wait time calculation).
    pub last_enqueue_tick: u64,
    /// Timestamp when task was last scheduled to run.
    pub last_run_tick: u64,
}
