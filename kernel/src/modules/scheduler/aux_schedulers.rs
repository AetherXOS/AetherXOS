use super::{lottery_base_tickets_from_raw, Scheduler, MAX_TASKS};

/// Lottery: Probabilistic fair scheduler. Each task gets tickets proportional
/// to its priority. A random ticket is drawn each scheduling decision.
pub struct LotteryScheduler {
    tasks: [(Option<usize>, u32); MAX_TASKS], // (task_id, tickets)
    count: usize,
    current_task: Option<usize>,
    prng_state: u64,
    slice_counter: u64,
}

impl Default for LotteryScheduler {
    fn default() -> Self {
        Self {
            tasks: [(None, 0); MAX_TASKS],
            count: 0,
            current_task: None,
            prng_state: crate::config::KernelConfig::sched_lottery_initial_seed(),
            slice_counter: 0,
        }
    }
}

impl LotteryScheduler {
    #[inline(always)]
    fn base_tickets() -> u32 {
        lottery_base_tickets_from_raw(
            crate::config::KernelConfig::sched_lottery_min_tickets_per_task(),
        )
    }

    fn xorshift64(&mut self) -> u64 {
        let mut x = self.prng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.prng_state = x;
        x
    }
}

impl Scheduler for LotteryScheduler {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            return;
        }
        let tickets = Self::base_tickets();
        self.tasks[self.count] = (Some(task_id), tickets);
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        if let Some(task) = self.current_task {
            if self.count < MAX_TASKS {
                self.tasks[self.count] = (Some(task), Self::base_tickets());
                self.count += 1;
            }
        }

        if self.count == 0 {
            self.current_task = None;
            return None;
        }

        // Count total tickets
        let total: u32 = self.tasks[..self.count].iter().map(|(_, t)| *t).sum();

        if total == 0 {
            self.current_task = self.tasks[0].0;
            return self.current_task;
        }

        // Draw random ticket
        let winner = (self.xorshift64() % total as u64) as u32;
        let mut cumulative = 0u32;
        let mut winner_idx = 0;

        for i in 0..self.count {
            cumulative += self.tasks[i].1;
            if cumulative > winner {
                winner_idx = i;
                break;
            }
        }

        let (task, _) = self.tasks[winner_idx];
        self.tasks[winner_idx] = self.tasks[self.count - 1];
        self.tasks[self.count - 1] = (None, 0);
        self.count -= 1;

        self.current_task = task;
        self.slice_counter = 0;
        task
    }

    fn tick(&mut self) -> bool {
        self.slice_counter += crate::generated_consts::TIME_SLICE_NS;
        self.slice_counter >= crate::config::KernelConfig::time_slice()
    }

    fn yield_now(&mut self) {
        self.slice_counter = u64::MAX; // Force reschedule
    }
}

/// Idle: Only runs when no other scheduler has tasks. Executes HLT-based idle.
pub struct IdleScheduler {
    idle_task: Option<usize>,
}

impl Default for IdleScheduler {
    fn default() -> Self {
        Self { idle_task: None }
    }
}

impl Scheduler for IdleScheduler {
    fn add_task(&mut self, task_id: usize) {
        self.idle_task = Some(task_id);
    }

    fn schedule(&mut self) -> Option<usize> {
        self.idle_task
    }

    fn tick(&mut self) -> bool {
        // Idle tasks never preempt - external events wake higher-priority schedulers
        false
    }

    fn yield_now(&mut self) {}
}

/// UserSpaceDelegator: Forwards scheduling decisions to a user-space handler.
/// The kernel stores pending tasks and lets userspace pick the winner via syscall.
pub struct UserSpaceDelegator {
    pending_tasks: [Option<usize>; MAX_TASKS],
    count: usize,
    chosen_task: Option<usize>,
}

impl Default for UserSpaceDelegator {
    fn default() -> Self {
        Self {
            pending_tasks: [None; MAX_TASKS],
            count: 0,
            chosen_task: None,
        }
    }
}

impl Scheduler for UserSpaceDelegator {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            return;
        }
        self.pending_tasks[self.count] = Some(task_id);
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        // If userspace has made a choice, use it
        if let Some(chosen) = self.chosen_task.take() {
            // Remove from pending
            for i in 0..self.count {
                if self.pending_tasks[i] == Some(chosen) {
                    self.pending_tasks[i] = self.pending_tasks[self.count - 1];
                    self.pending_tasks[self.count - 1] = None;
                    self.count -= 1;
                    return Some(chosen);
                }
            }
        }
        // Fallback: FIFO if userspace hasn't decided
        if self.count > 0 {
            let task = self.pending_tasks[0];
            for i in 0..self.count - 1 {
                self.pending_tasks[i] = self.pending_tasks[i + 1];
            }
            self.pending_tasks[self.count - 1] = None;
            self.count -= 1;
            return task;
        }
        None
    }

    fn tick(&mut self) -> bool {
        // Reschedule every tick to check if userspace made a decision
        self.count > 0
    }

    fn yield_now(&mut self) {}
}

impl UserSpaceDelegator {
    /// Called from syscall layer when userspace chooses a task to run
    pub fn set_chosen(&mut self, task_id: usize) {
        self.chosen_task = Some(task_id);
    }

    /// Return snapshot of pending task IDs for userspace to inspect
    pub fn pending_snapshot(&self) -> &[Option<usize>] {
        &self.pending_tasks[..self.count]
    }
}
