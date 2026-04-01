use super::{Scheduler, MAX_TASKS};

/// EDF: Real-time scheduler that always runs the task with earliest absolute deadline.
pub struct EdfScheduler {
    tasks: [(Option<usize>, u64); MAX_TASKS], // (task_id, absolute_deadline)
    count: usize,
    current_task: Option<usize>,
    current_deadline: u64,
}

impl Default for EdfScheduler {
    fn default() -> Self {
        Self {
            tasks: [(None, 0); MAX_TASKS],
            count: 0,
            current_task: None,
            current_deadline: u64::MAX,
        }
    }
}

impl Scheduler for EdfScheduler {
    fn add_task(&mut self, task_id: usize) {
        if self.count >= MAX_TASKS {
            return;
        }
        // Deadline stored in upper bits of task_id, or use a fixed default
        let deadline = crate::kernel::watchdog::global_tick() + 50_000_000; // 50ms deadline
        self.tasks[self.count] = (Some(task_id), deadline);
        self.count += 1;
    }

    fn schedule(&mut self) -> Option<usize> {
        if let Some(task) = self.current_task {
            if self.count < MAX_TASKS {
                self.tasks[self.count] = (Some(task), self.current_deadline);
                self.count += 1;
            }
        }

        if self.count == 0 {
            self.current_task = None;
            return None;
        }

        // Find earliest deadline
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
        let now = crate::kernel::watchdog::global_tick();
        // Preempt if a task with earlier deadline exists
        for i in 0..self.count {
            if let (Some(_), dl) = self.tasks[i] {
                if dl < self.current_deadline {
                    return true;
                }
            }
        }
        // Also preempt if current task missed its deadline
        now > self.current_deadline
    }

    fn yield_now(&mut self) {}
}
