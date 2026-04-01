use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;

pub struct WeightedRoundRobin {
    // Map Priority -> Queue of TaskIds
    queues: BTreeMap<u8, VecDeque<TaskId>>,
    tasks: BTreeMap<TaskId, Arc<IrqSafeMutex<KernelTask>>>,
}

impl WeightedRoundRobin {
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            tasks: BTreeMap::new(),
        }
    }

    #[inline(always)]
    fn priority_weight(task: &Arc<IrqSafeMutex<KernelTask>>) -> (TaskId, u8) {
        let guard = task.lock();
        let tid = guard.id;
        let weight = (guard.priority as usize).min(crate::generated_consts::WRR_MAX_WEIGHT) as u8;
        (tid, weight)
    }

    fn drop_empty_queues(&mut self) {
        self.queues.retain(|_, q| !q.is_empty());
    }
}

impl Scheduler for WeightedRoundRobin {
    type TaskItem = Arc<IrqSafeMutex<KernelTask>>;

    fn runqueue_len(&self) -> usize {
        self.tasks.len()
    }

    fn init(&mut self) {}

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.tasks.get_mut(&task_id)
    }

    fn add_task(&mut self, task: Self::TaskItem) {
        let (tid, weight) = Self::priority_weight(&task);

        self.queues
            .entry(weight)
            .or_insert(VecDeque::new())
            .push_back(tid);
        self.tasks.insert(tid, task);
    }

    fn remove_task(&mut self, task_id: TaskId) {
        self.tasks.remove(&task_id);
        for queue in self.queues.values_mut() {
            if let Some(pos) = queue.iter().position(|&id| id == task_id) {
                queue.remove(pos);
                self.drop_empty_queues();
                return;
            }
        }
    }

    fn remove_task_item(&mut self, task_id: TaskId) -> Option<Self::TaskItem> {
        let item = self.tasks.remove(&task_id)?;
        for queue in self.queues.values_mut() {
            if let Some(pos) = queue.iter().position(|&id| id == task_id) {
                queue.remove(pos);
                break;
            }
        }
        self.drop_empty_queues();
        Some(item)
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        // Iterate from highest priority (lower numbers = higher priority in BTreeMap by default)
        for (_prio, queue) in self.queues.iter_mut() {
            if let Some(tid) = queue.pop_front() {
                queue.push_back(tid); // Round-robin within priority
                return Some(tid);
            }
        }
        None
    }

    fn tick(&mut self, _current: TaskId) -> SchedulerAction {
        SchedulerAction::Reschedule
    }
}
