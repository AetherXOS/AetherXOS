use crate::config::KernelConfig;
use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct EdfRuntimeStats {
    pub ticks: u64,
    pub deadline_misses: u64,
    pub reschedule_hints: u64,
    pub window_resets: u64,
    pub group_throttle_events: u64,
}

static EDF_TICKS: AtomicU64 = AtomicU64::new(0);
static EDF_DEADLINE_MISSES: AtomicU64 = AtomicU64::new(0);
static EDF_RESCHEDULE_HINTS: AtomicU64 = AtomicU64::new(0);
static EDF_WINDOW_RESETS: AtomicU64 = AtomicU64::new(0);
static EDF_GROUP_THROTTLE_EVENTS: AtomicU64 = AtomicU64::new(0);

/// Earliest Deadline First (EDF) - Real-Time Scheduler.
/// Hard Real-Time: Always picks the task with the closest deadline.
/// Used in audio processing, industrial control, avionics.

pub struct EDF {
    queue: Vec<Arc<IrqSafeMutex<KernelTask>>>,
    elapsed_ns: u64,
    rt_window_start_ns: u64,
    rt_window_used_ns: u64,
    rt_group_used_ns: BTreeMap<u16, u64>,
}

impl EDF {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            elapsed_ns: 0,
            rt_window_start_ns: 0,
            rt_window_used_ns: 0,
            rt_group_used_ns: BTreeMap::new(),
        }
    }

    fn effective_deadline_ns(&self, task: &KernelTask) -> u64 {
        if task.deadline == 0 {
            return self
                .elapsed_ns
                .saturating_add(KernelConfig::edf_default_relative_deadline_ns());
        }
        task.deadline
    }

    fn refresh_order(&mut self) {
        let now = self.elapsed_ns;
        let default_rel = KernelConfig::edf_default_relative_deadline_ns();
        self.queue.sort_by_key(|task_handle| {
            let task = task_handle.lock();
            if task.deadline == 0 {
                now.saturating_add(default_rel)
            } else {
                task.deadline
            }
        });
    }

    fn reset_rt_window_if_needed(&mut self) {
        let window = KernelConfig::rt_period_ns().max(KernelConfig::time_slice());
        if self.elapsed_ns.saturating_sub(self.rt_window_start_ns) >= window {
            self.rt_window_start_ns = self.elapsed_ns;
            self.rt_window_used_ns = 0;
            self.rt_group_used_ns.clear();
            EDF_WINDOW_RESETS.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn rt_group_allows(&self, task_handle: &Arc<IrqSafeMutex<KernelTask>>) -> bool {
        if !KernelConfig::rt_group_reservation_enabled() {
            return true;
        }

        let task = task_handle.lock();
        let window = KernelConfig::rt_period_ns().max(KernelConfig::time_slice());
        let total_cap = (window as u128)
            .saturating_mul(KernelConfig::rt_total_utilization_cap_percent() as u128)
            / 100;
        let max_groups = KernelConfig::rt_max_groups().max(1) as u128;
        let per_group_cap = (total_cap / max_groups) as u64;
        let dispatch_cost = KernelConfig::time_slice();

        let current_group_used = *self.rt_group_used_ns.get(&task.rt_group_id).unwrap_or(&0);
        let next_group_used = current_group_used.saturating_add(dispatch_cost);
        let next_total_used = self.rt_window_used_ns.saturating_add(dispatch_cost);

        next_group_used <= per_group_cap && (next_total_used as u128) <= total_cap
    }

    fn account_dispatch(&mut self, task_handle: &Arc<IrqSafeMutex<KernelTask>>) {
        if !KernelConfig::rt_group_reservation_enabled() {
            return;
        }
        let task = task_handle.lock();
        self.rt_window_used_ns = self
            .rt_window_used_ns
            .saturating_add(KernelConfig::time_slice());
        let entry = self.rt_group_used_ns.entry(task.rt_group_id).or_insert(0);
        *entry = entry.saturating_add(KernelConfig::time_slice());
    }
}

impl Scheduler for EDF {
    type TaskItem = Arc<IrqSafeMutex<KernelTask>>;

    fn init(&mut self) {}

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.queue.iter_mut().find(|t| t.lock().id == task_id)
    }

    fn add_task(&mut self, task_handle: Self::TaskItem) {
        if KernelConfig::rt_group_reservation_enabled() {
            let mut task = task_handle.lock();
            let max_group_id = KernelConfig::rt_max_groups().saturating_sub(1) as u16;
            if task.rt_group_id > max_group_id {
                task.rt_group_id = max_group_id;
            }
        }
        self.queue.push(task_handle);
        self.refresh_order();
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
            .and_then(|pos| Some(self.queue.remove(pos)))
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        self.reset_rt_window_if_needed();
        self.refresh_order();

        for task_handle in &self.queue {
            if self.rt_group_allows(task_handle) {
                return Some(task_handle.lock().id);
            }
            EDF_GROUP_THROTTLE_EVENTS.fetch_add(1, Ordering::Relaxed);
        }
        self.queue.first().map(|t| t.lock().id)
    }

    fn tick(&mut self, current: TaskId) -> SchedulerAction {
        EDF_TICKS.fetch_add(1, Ordering::Relaxed);
        self.elapsed_ns = self.elapsed_ns.saturating_add(KernelConfig::time_slice());
        self.reset_rt_window_if_needed();

        if let Some(pos) = self.queue.iter().position(|t| t.lock().id == current) {
            let task_handle = self.queue[pos].clone();
            self.account_dispatch(&task_handle);
            if KernelConfig::edf_enforce_deadline() {
                let task = task_handle.lock();
                let deadline = self.effective_deadline_ns(&task);
                if self.elapsed_ns > deadline {
                    EDF_DEADLINE_MISSES.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        self.refresh_order();

        // In EDF, if a new task arrives with earlier deadline, we preempt.
        // Or if current task finishes.
        // Here we just check if the current task is still the one with earliest deadline.
        // Since we sort on insert, the head is always the target.
        if let Some(task_handle) = self.queue.first() {
            let head = task_handle.lock();
            if head.id != current {
                EDF_RESCHEDULE_HINTS.fetch_add(1, Ordering::Relaxed);
                return SchedulerAction::Reschedule;
            }

            if KernelConfig::edf_enforce_deadline() {
                let head_deadline = self.effective_deadline_ns(&head);
                if self.elapsed_ns > head_deadline {
                    EDF_RESCHEDULE_HINTS.fetch_add(1, Ordering::Relaxed);
                    return SchedulerAction::Reschedule;
                }
            }
        }
        // Also check if deadline missed? (Hard RT failure)
        SchedulerAction::Continue
    }
}

pub fn runtime_stats() -> EdfRuntimeStats {
    EdfRuntimeStats {
        ticks: EDF_TICKS.load(Ordering::Relaxed),
        deadline_misses: EDF_DEADLINE_MISSES.load(Ordering::Relaxed),
        reschedule_hints: EDF_RESCHEDULE_HINTS.load(Ordering::Relaxed),
        window_resets: EDF_WINDOW_RESETS.load(Ordering::Relaxed),
        group_throttle_events: EDF_GROUP_THROTTLE_EVENTS.load(Ordering::Relaxed),
    }
}
