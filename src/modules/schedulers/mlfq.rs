use crate::config::KernelConfig;
use crate::generated_consts::{SCHED_MLFQ_NUM_QUEUES, TIME_SLICE_NS};
use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;

/// Multi-Level Feedback Queue (MLFQ) Scheduler.
/// O(1) scheduling complexity.
///
/// - Multiple priority queues (0 = Highest, N = Lowest).
/// - Tasks start at highest priority.
/// - If a task uses its entire time slice, it drops a priority level (CPU-bound penalty).
/// - If a task yields before slice ends, it stays same or ascends (I/O-bound reward).
/// - Periodic "Boost" moves everyone to top to prevent starvation.

const NUM_QUEUES: usize = SCHED_MLFQ_NUM_QUEUES;

pub struct MLFQ {
    queues: [VecDeque<TaskId>; NUM_QUEUES],
    tasks: BTreeMap<TaskId, Arc<IrqSafeMutex<KernelTask>>>,
    task_info: BTreeMap<TaskId, (usize, u64)>, // (QueueIndex, TimeUsedInSlice)
    boost_ticks: u64,
}

impl MLFQ {
    pub fn new() -> Self {
        const INIT: VecDeque<TaskId> = VecDeque::new();
        Self {
            queues: [INIT; NUM_QUEUES],
            tasks: BTreeMap::new(),
            task_info: BTreeMap::new(),
            boost_ticks: 0,
        }
    }

    fn boost_all(&mut self) {
        for i in 1..NUM_QUEUES {
            while let Some(task) = self.queues[i].pop_front() {
                self.queues[0].push_back(task);
                if let Some(info) = self.task_info.get_mut(&task) {
                    info.0 = 0;
                    info.1 = 0;
                }
            }
        }
    }
}

impl Scheduler for MLFQ {
    type TaskItem = Arc<IrqSafeMutex<KernelTask>>;

    fn runqueue_len(&self) -> usize {
        self.tasks.len()
    }

    fn init(&mut self) {}

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.tasks.get_mut(&task_id)
    }

    fn add_task(&mut self, task: Self::TaskItem) {
        let tid = task.lock().id;
        self.queues[0].push_back(tid);
        self.task_info.insert(tid, (0, 0));
        self.tasks.insert(tid, task);
    }

    fn remove_task(&mut self, task_id: TaskId) {
        self.tasks.remove(&task_id);
        self.task_info.remove(&task_id);
        for q in self.queues.iter_mut() {
            if let Some(pos) = q.iter().position(|&id| id == task_id) {
                q.remove(pos);
                return;
            }
        }
    }

    fn remove_task_item(&mut self, task_id: TaskId) -> Option<Self::TaskItem> {
        let item = self.tasks.remove(&task_id)?;
        self.task_info.remove(&task_id);
        for q in self.queues.iter_mut() {
            if let Some(pos) = q.iter().position(|&id| id == task_id) {
                q.remove(pos);
                break;
            }
        }
        Some(item)
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        // Scan queues from highest priority (0) to lowest
        for q in self.queues.iter_mut() {
            if let Some(task_id) = q.pop_front() {
                q.push_back(task_id); // Round-robin within the same priority queue.
                return Some(task_id);
            }
        }
        None
    }

    fn tick(&mut self, current: TaskId) -> SchedulerAction {
        // 1. Starvation Avoidance Boost
        self.boost_ticks = self.boost_ticks.saturating_add(1);
        if self.boost_ticks >= KernelConfig::mlfq_boost_interval_ticks() {
            self.boost_all();
            self.boost_ticks = 0;
            return SchedulerAction::Reschedule;
        }

        // 2. Accounting
        if let Some(info) = self.task_info.get_mut(&current) {
            let q_idx = info.0;
            info.1 = info.1.saturating_add(TIME_SLICE_NS);

            // Slice limit increases with lower priority (longer bursts for background/CPU tasks)
            let limit = KernelConfig::mlfq_base_slice_ns() * ((q_idx as u64) + 1);

            if info.1 >= limit {
                info.1 = 0; // Reset slice usage

                // Demote if not at bottom
                if KernelConfig::mlfq_demote_on_slice_exhaustion() && q_idx < NUM_QUEUES - 1 {
                    // Find and remove from current queue
                    if let Some(pos) = self.queues[q_idx].iter().position(|&id| id == current) {
                        self.queues[q_idx].remove(pos);
                    }
                    // Add to next queue
                    let next_q = q_idx + 1;
                    self.queues[next_q].push_back(current);
                    info.0 = next_q; // Update stored level
                }

                return SchedulerAction::Reschedule;
            }
        }

        SchedulerAction::Continue
    }
}
