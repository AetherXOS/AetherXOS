use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::VecDeque;
use alloc::sync::Arc;

pub struct RoundRobin {
    queue: VecDeque<Arc<IrqSafeMutex<KernelTask>>>,
    time_slice_counter: u64,
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl RoundRobin {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            time_slice_counter: 0,
        }
    }
}

impl Scheduler for RoundRobin {
    type TaskItem = Arc<IrqSafeMutex<KernelTask>>;

    fn runqueue_len(&self) -> usize {
        self.queue.len()
    }

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.queue.iter_mut().find(|t| t.lock().id == task_id)
    }

    fn steal_task(&mut self) -> Option<Self::TaskItem> {
        self.queue.pop_back()
    }

    fn init(&mut self) {}

    fn add_task(&mut self, task: Self::TaskItem) {
        if self.queue.len() < crate::generated_consts::SCHED_RR_MAX_TASKS {
            self.queue.push_back(task);
        } else {
            let mut max_idx = 0;
            let mut max_time = 0u64;
            for (i, t) in self.queue.iter().enumerate() {
                let time = t.lock().time_consumed;
                if time >= max_time {
                    max_time = time;
                    max_idx = i;
                }
            }
            self.queue.remove(max_idx);
            self.queue.push_back(task);
        }
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
        self.queue.push_back(task);
        Some(id)
    }

    fn tick(&mut self, _current: TaskId) -> SchedulerAction {
        self.time_slice_counter += crate::generated_consts::TIME_SLICE_NS;
        if self.time_slice_counter >= crate::generated_consts::SCHED_RR_DEFAULT_SLICE_NS {
            self.time_slice_counter = 0;
            return SchedulerAction::Reschedule;
        }
        SchedulerAction::Continue
    }
}
