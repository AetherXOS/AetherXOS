use crate::interfaces::Scheduler;
#[path = "scheduler_support.rs"]
mod scheduler_support;
use scheduler_support::{
    cfs_should_preempt,
    fifo_priority_from_task,
    fifo_should_preempt,
    lottery_base_tickets_from_raw,
    next_round_robin_slice,
};
#[path = "scheduler/aux_schedulers.rs"]
mod aux_schedulers;
#[path = "scheduler/edf.rs"]
mod edf;

pub use aux_schedulers::{IdleScheduler, LotteryScheduler, UserSpaceDelegator};
pub use edf::EdfScheduler;

const MAX_TASKS: usize = 64;

// ─────────────────────────────── RoundRobin ──────────────────────────────

pub struct RoundRobin {
    time_slice: u64,
    tasks: [Option<usize>; MAX_TASKS],
    head: usize,
    tail: usize,
    count: usize,
    current_task: Option<usize>,
    slice_remaining: u64,
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self {
            time_slice: crate::config::KernelConfig::time_slice(),
            tasks: [None; MAX_TASKS],
            head: 0,
            tail: 0,
            count: 0,
            current_task: None,
            slice_remaining: 0,
        }
    }
}

impl RoundRobin {
    pub fn new(time_slice: u64) -> Self {
        Self {
            time_slice,
            ..Default::default()
        }
    }
}

impl Scheduler for RoundRobin {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            // Queue full — evict the oldest task to make room
            self.tasks[self.head] = None;
            self.head = (self.head + 1) % MAX_TASKS;
            self.count -= 1;
        }
        self.tasks[self.tail] = Some(task_id);
        self.tail = (self.tail + 1) % MAX_TASKS;
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        if let Some(current) = self.current_task {
            self.add_task(current);
        }

        if self.count == 0 {
            self.current_task = None;
            return None;
        }

        let next_task = self.tasks[self.head];
        self.tasks[self.head] = None;
        self.head = (self.head + 1) % MAX_TASKS;
        self.count -= 1;

        self.current_task = next_task;
        self.slice_remaining = self.time_slice;
        next_task
    }

    fn tick(&mut self) -> bool {
        // Decrement time slice and reschedule when exhausted
        let tick_ns = crate::generated_consts::TIME_SLICE_NS;
        let (next_slice, should_reschedule) = next_round_robin_slice(self.slice_remaining, tick_ns);
        self.slice_remaining = next_slice;
        should_reschedule
    }

    fn yield_now(&mut self) {
        self.slice_remaining = 0;
    }
}

// ─────────────────────────────── FIFO Real-Time ──────────────────────────

/// FIFO Real-Time Scheduler: tasks run to completion or until they yield.
/// Higher priority tasks always preempt lower ones.
pub struct FIFORealTime {
    tasks: [(Option<usize>, u8); MAX_TASKS], // (task_id, priority)
    count: usize,
    current_task: Option<usize>,
    current_priority: u8,
}

impl Default for FIFORealTime {
    fn default() -> Self {
        Self {
            tasks: [(None, 0); MAX_TASKS],
            count: 0,
            current_task: None,
            current_priority: 0,
        }
    }
}

impl Scheduler for FIFORealTime {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            return;
        }
        // Insert sorted by priority (higher priority = lower index)
        let priority = fifo_priority_from_task(task_id); // extract priority from task hint
        let mut insert_pos = self.count;
        for i in 0..self.count {
            if let Some(_) = self.tasks[i].0 {
                if fifo_should_preempt(priority, self.tasks[i].1) {
                    insert_pos = i;
                    break;
                }
            }
        }
        // Shift right
        if insert_pos < self.count {
            for i in (insert_pos..self.count).rev() {
                self.tasks[i + 1] = self.tasks[i];
            }
        }
        self.tasks[insert_pos] = (Some(task_id), priority);
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        if self.count == 0 {
            self.current_task = None;
            return None;
        }
        let (task, prio) = self.tasks[0];
        // Shift left to remove
        for i in 0..self.count - 1 {
            self.tasks[i] = self.tasks[i + 1];
        }
        self.tasks[self.count - 1] = (None, 0);
        self.count -= 1;
        self.current_task = task;
        self.current_priority = prio;
        task
    }

    fn tick(&mut self) -> bool {
        // FIFO: don't preempt based on time, only on priority
        // Check if any waiting task has higher priority
        if self.count > 0 {
            if let Some((_, prio)) = self.tasks.first() {
                if fifo_should_preempt(*prio, self.current_priority) {
                    return true; // Higher priority task waiting
                }
            }
        }
        false
    }

    fn yield_now(&mut self) {
        // Re-enqueue current task at same priority
        if let Some(task) = self.current_task {
            self.add_task(task);
        }
    }
}

// ───────────────────────── CFS (Completely Fair) ─────────────────────────

/// CFS: Uses virtual runtime to ensure fairness.
/// Tasks with less CPU time get scheduled first.
pub struct CFSScheduler {
    tasks: [(Option<usize>, u64); MAX_TASKS], // (task_id, vruntime)
    count: usize,
    current_task: Option<usize>,
    current_vruntime: u64,
    min_vruntime: u64,
}

impl Default for CFSScheduler {
    fn default() -> Self {
        Self {
            tasks: [(None, 0); MAX_TASKS],
            count: 0,
            current_task: None,
            current_vruntime: 0,
            min_vruntime: 0,
        }
    }
}

impl Scheduler for CFSScheduler {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            return;
        }
        self.tasks[self.count] = (Some(task_id), self.min_vruntime);
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        // Re-enqueue current task with updated vruntime
        if let Some(task) = self.current_task {
            if self.count < MAX_TASKS {
                self.tasks[self.count] = (Some(task), self.current_vruntime);
                self.count += 1;
            }
        }

        if self.count == 0 {
            self.current_task = None;
            return None;
        }

        // Find task with minimum vruntime
        let mut min_idx = 0;
        let mut min_vrt = u64::MAX;
        for i in 0..self.count {
            if let (Some(_), vrt) = self.tasks[i] {
                if vrt < min_vrt {
                    min_vrt = vrt;
                    min_idx = i;
                }
            }
        }

        let (task, vrt) = self.tasks[min_idx];
        // Remove by swapping with last
        self.tasks[min_idx] = self.tasks[self.count - 1];
        self.tasks[self.count - 1] = (None, 0);
        self.count -= 1;

        self.current_task = task;
        self.current_vruntime = vrt;
        self.min_vruntime = vrt;
        task
    }

    fn tick(&mut self) -> bool {
        let tick_ns = crate::generated_consts::TIME_SLICE_NS;
        self.current_vruntime = self.current_vruntime.saturating_add(tick_ns);

        // Check if any task has significantly lower vruntime
        let granularity = crate::generated_consts::SCHED_CFS_MIN_GRANULARITY_NS;
        cfs_should_preempt(self.current_vruntime, &self.tasks[..self.count], granularity)
    }

    fn yield_now(&mut self) {}
}

// ──────────────────────────── MuQSS (Gaming) ─────────────────────────────

/// MuQSS: Multiple Queue Skiplist Scheduler — optimized for low latency.
/// Uses a deadline-based approach where tasks get virtual deadlines.
pub struct MuQSSScheduler {
    tasks: [(Option<usize>, u64); MAX_TASKS], // (task_id, virtual_deadline)
    count: usize,
    current_task: Option<usize>,
    current_deadline: u64,
    global_tick: u64,
}

impl Default for MuQSSScheduler {
    fn default() -> Self {
        Self {
            tasks: [(None, 0); MAX_TASKS],
            count: 0,
            current_task: None,
            current_deadline: 0,
            global_tick: 0,
        }
    }
}

impl Scheduler for MuQSSScheduler {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            return;
        }
        // Virtual deadline = now + base_slice (shorter for interactive tasks)
        let deadline = self.global_tick + 6_000_000; // 6ms deadline
        self.tasks[self.count] = (Some(task_id), deadline);
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        if let Some(task) = self.current_task {
            if self.count < MAX_TASKS {
                // Re-insert with new deadline
                let new_deadline = self.global_tick + 6_000_000;
                self.tasks[self.count] = (Some(task), new_deadline);
                self.count += 1;
            }
        }

        if self.count == 0 {
            self.current_task = None;
            return None;
        }

        // Pick task with earliest deadline
        let mut earliest_idx = 0;
        let mut earliest_dl = u64::MAX;
        for i in 0..self.count {
            if let (Some(_), dl) = self.tasks[i] {
                if dl < earliest_dl {
                    earliest_dl = dl;
                    earliest_idx = i;
                }
            }
        }

        let (task, dl) = self.tasks[earliest_idx];
        self.tasks[earliest_idx] = self.tasks[self.count - 1];
        self.tasks[self.count - 1] = (None, 0);
        self.count -= 1;

        self.current_task = task;
        self.current_deadline = dl;
        task
    }

    fn tick(&mut self) -> bool {
        self.global_tick = self.global_tick.saturating_add(crate::generated_consts::TIME_SLICE_NS);
        // Preempt if deadline passed
        self.global_tick >= self.current_deadline
    }

    fn yield_now(&mut self) {
        self.current_deadline = 0; // Force reschedule
    }
}

// ──────────────────────── EEVDS (Lag Reduction) ──────────────────────────

/// EEVDS: Earliest Eligible Virtual Deadline Scheduler.
/// Combines virtual time and eligibility to reduce lag for interactive tasks.
pub struct EEVDSScheduler {
    tasks: [(Option<usize>, u64, u64); MAX_TASKS], // (task_id, eligible_time, virtual_deadline)
    count: usize,
    current_task: Option<usize>,
    global_virtual_time: u64,
}

impl Default for EEVDSScheduler {
    fn default() -> Self {
        Self {
            tasks: [(None, 0, 0); MAX_TASKS],
            count: 0,
            current_task: None,
            global_virtual_time: 0,
        }
    }
}

impl Scheduler for EEVDSScheduler {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            return;
        }
        let eligible = self.global_virtual_time;
        let deadline = eligible + 8_000_000; // 8ms virtual deadline
        self.tasks[self.count] = (Some(task_id), eligible, deadline);
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        if let Some(task) = self.current_task {
            if self.count < MAX_TASKS {
                let eligible = self.global_virtual_time;
                let deadline = eligible + 8_000_000;
                self.tasks[self.count] = (Some(task), eligible, deadline);
                self.count += 1;
            }
        }

        if self.count == 0 {
            self.current_task = None;
            return None;
        }

        // Find eligible tasks (eligible_time <= global_virtual_time), then pick earliest deadline
        let mut best_idx = None;
        let mut best_deadline = u64::MAX;
        for i in 0..self.count {
            if let (Some(_), eligible, deadline) = self.tasks[i] {
                if eligible <= self.global_virtual_time && deadline < best_deadline {
                    best_deadline = deadline;
                    best_idx = Some(i);
                }
            }
        }

        // If no eligible task, pick the one closest to being eligible
        if best_idx.is_none() {
            let mut closest_eligible = u64::MAX;
            for i in 0..self.count {
                if let (Some(_), eligible, _) = self.tasks[i] {
                    if eligible < closest_eligible {
                        closest_eligible = eligible;
                        best_idx = Some(i);
                    }
                }
            }
        }

        let idx = best_idx?;
        let (task, _, _) = self.tasks[idx];
        self.tasks[idx] = self.tasks[self.count - 1];
        self.tasks[self.count - 1] = (None, 0, 0);
        self.count -= 1;

        self.current_task = task;
        task
    }

    fn tick(&mut self) -> bool {
        self.global_virtual_time = self.global_virtual_time.saturating_add(
            crate::generated_consts::TIME_SLICE_NS
        );
        // Preempt if a higher-priority eligible task exists
        for i in 0..self.count {
            if let (Some(_), eligible, _) = self.tasks[i] {
                if eligible <= self.global_virtual_time {
                    return true;
                }
            }
        }
        false
    }

    fn yield_now(&mut self) {}
}

// ─────────────────────── Cooperative Scheduler ───────────────────────────

/// Cooperative: No preemption. Tasks run until they explicitly yield.
pub struct CooperativeScheduler {
    tasks: [Option<usize>; MAX_TASKS],
    count: usize,
    head: usize,
    current_task: Option<usize>,
    yield_requested: bool,
}

impl Default for CooperativeScheduler {
    fn default() -> Self {
        Self {
            tasks: [None; MAX_TASKS],
            count: 0,
            head: 0,
            current_task: None,
            yield_requested: false,
        }
    }
}

impl Scheduler for CooperativeScheduler {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            return;
        }
        let tail = (self.head + self.count) % MAX_TASKS;
        self.tasks[tail] = Some(task_id);
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        if self.yield_requested {
            if let Some(current) = self.current_task {
                self.add_task(current);
            }
            self.yield_requested = false;
        }

        if self.count == 0 {
            self.current_task = None;
            return None;
        }

        let task = self.tasks[self.head];
        self.tasks[self.head] = None;
        self.head = (self.head + 1) % MAX_TASKS;
        self.count -= 1;

        self.current_task = task;
        task
    }

    fn tick(&mut self) -> bool {
        // Cooperative: never preempt on tick
        false
    }

    fn yield_now(&mut self) {
        self.yield_requested = true;
    }
}


