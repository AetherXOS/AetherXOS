use crate::interfaces::task::TaskId;

/// The core Scheduler Interface (LSP)
/// All 15 variants must support this.
pub trait Scheduler {
    type TaskItem: Send + Sync;

    fn get_task_mut(&mut self, _task_id: TaskId) -> Option<&mut Self::TaskItem> {
        None
    }
    fn steal_task(&mut self) -> Option<Self::TaskItem> {
        None
    }
    fn runqueue_len(&self) -> usize {
        0
    }
    fn cpu_load(&self) -> usize {
        self.runqueue_len()
    }

    fn init(&mut self);
    fn add_task(&mut self, task: Self::TaskItem);
    fn remove_task(&mut self, task_id: TaskId);
    fn remove_task_item(&mut self, task_id: TaskId) -> Option<Self::TaskItem>;
    fn pick_next(&mut self) -> Option<TaskId>;
    fn bootstrap_pick_next(&mut self) -> Option<TaskId> {
        self.pick_next()
    }
    fn tick(&mut self, current_task: TaskId) -> SchedulerAction;
}

#[derive(Debug, PartialEq)]
pub enum SchedulerAction {
    Continue,
    Reschedule,
    Yield,
}
