use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::VecDeque;
use alloc::sync::Arc;

pub struct FIFO {
    queue: VecDeque<Arc<IrqSafeMutex<KernelTask>>>,
}

impl FIFO {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl Scheduler for FIFO {
    type TaskItem = Arc<IrqSafeMutex<KernelTask>>;

    fn runqueue_len(&self) -> usize {
        self.queue.len()
    }

    fn init(&mut self) {}

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.queue.iter_mut().find(|t| t.lock().id == task_id)
    }

    fn add_task(&mut self, task: Self::TaskItem) {
        self.queue.push_back(task);
    }

    fn remove_task(&mut self, task_id: TaskId) {
        if let Some(pos) = self.queue.iter().position(|t| t.lock().id == task_id) {
            self.queue.remove(pos);
        }
    }

    fn remove_task_item(&mut self, task_id: TaskId) -> Option<Self::TaskItem> {
        self.queue
            .iter()
            .position(|t| t.lock().id == task_id)
            .and_then(|pos| self.queue.remove(pos))
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        let task = self.queue.pop_front()?;
        let id = task.lock().id;
        self.queue.push_back(task); // Standard behavior for this loop-style runner
        Some(id)
    }

    fn tick(&mut self, _current: TaskId) -> SchedulerAction {
        if crate::generated_consts::FIFO_ALLOW_PREEMPTION {
            return SchedulerAction::Reschedule;
        }
        SchedulerAction::Continue
    }
}
