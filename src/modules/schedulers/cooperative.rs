use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use alloc::collections::VecDeque;

/// Cooperative Scheduler.
/// Never preempts. Tasks must voluntarily yield.
/// Very low overhead (no timer interrupts needed).

pub struct Cooperative {
    queue: VecDeque<KernelTask>,
}

impl Cooperative {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl Scheduler for Cooperative {
    type TaskItem = KernelTask;

    fn runqueue_len(&self) -> usize {
        self.queue.len()
    }

    fn init(&mut self) {}

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.queue.iter_mut().find(|t| t.id == task_id)
    }

    fn add_task(&mut self, task: Self::TaskItem) {
        self.queue.push_back(task);
    }

    fn remove_task(&mut self, task_id: TaskId) {
        if let Some(pos) = self.queue.iter().position(|t| t.id == task_id) {
            self.queue.remove(pos);
        }
    }

    fn remove_task_item(&mut self, task_id: TaskId) -> Option<Self::TaskItem> {
        self.queue
            .iter()
            .position(|t| t.id == task_id)
            .and_then(|pos| self.queue.remove(pos))
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        self.queue.front().map(|t| t.id)
    }

    fn tick(&mut self, _current: TaskId) -> SchedulerAction {
        SchedulerAction::Continue
    }
}
