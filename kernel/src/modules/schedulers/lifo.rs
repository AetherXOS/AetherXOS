use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use alloc::vec::Vec;

/// Last-In, First-Out (LIFO) Scheduler.
/// Useful for cache maximization in certain batch processing workloads.
/// "Stack" based scheduling.

pub struct LIFO {
    stack: Vec<TaskId>,
    tasks: alloc::collections::BTreeMap<TaskId, KernelTask>,
}

impl LIFO {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            tasks: alloc::collections::BTreeMap::new(),
        }
    }
}

impl Scheduler for LIFO {
    type TaskItem = KernelTask;

    fn init(&mut self) {}

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.tasks.get_mut(&task_id)
    }

    fn add_task(&mut self, task: Self::TaskItem) {
        let tid = task.id;
        self.stack.push(tid);
        self.tasks.insert(tid, task);
    }

    fn remove_task(&mut self, task_id: TaskId) {
        self.tasks.remove(&task_id);
        if let Some(pos) = self.stack.iter().position(|&id| id == task_id) {
            self.stack.remove(pos);
        }
    }

    fn remove_task_item(&mut self, task_id: TaskId) -> Option<Self::TaskItem> {
        let task = self.tasks.remove(&task_id)?;
        if let Some(pos) = self.stack.iter().position(|&id| id == task_id) {
            self.stack.remove(pos);
        }
        Some(task)
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        self.stack.last().copied()
    }

    fn tick(&mut self, _current: TaskId) -> SchedulerAction {
        SchedulerAction::Continue
    }
}
