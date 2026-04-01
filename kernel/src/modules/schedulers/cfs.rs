//! CFS (Completely Fair Scheduler) - Production Grade.
//!
//! Uses a Red-Black Tree (BTreeMap) to order tasks by `vruntime`.
//! Implements "Weighted Fair Queuing" where tasks with higher priority (weight)
//! accumulate `vruntime` slower, thus getting more CPU time.
//! Uses the standard Linux priority-to-weight table for nice values [-20, 19].
//!
//! Supports hierarchical group scheduling: tasks are assigned to CFS groups,
//! and the scheduler first picks the group with the lowest group-level vruntime,
//! then picks the task within that group. Groups can have CPU quota enforcement.

use crate::config::KernelConfig;
use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
#[path = "cfs_support.rs"]
mod cfs_support;
use cfs_support::{
    calc_delta_vruntime, calculate_weight, group_is_throttled, should_preempt_current,
    NICE_0_LOAD,
};
mod autogroup;
mod topology;

pub use autogroup::Autogroup;
pub use topology::{CpuTopology, CpuTopologyEntry, SchedDomainLevel};

const CGROUP_CPU_PERIOD_US: u64 = 100_000;
/// Maximum number of CFS groups supported.
const MAX_CFS_GROUPS: usize = 64;

/// CFS group for hierarchical scheduling.
/// Each group has its own vruntime timeline and a proportional weight
/// used at the top-level scheduling decision.
#[derive(Debug)]
struct CfsGroup {
    /// Group identifier (0 = root group).
    _id: u16,
    /// Aggregate weight of all tasks in this group.
    total_weight: u64,
    /// Group-level vruntime (advances proportionally to group weight).
    vruntime: u64,
    /// CPU quota in nanoseconds per period (0 = unlimited).
    cpu_quota_ns: u64,
    /// CPU consumed in current period.
    cpu_used_ns: u64,
    /// Number of tasks in this group.
    nr_tasks: usize,
}

impl CfsGroup {
    fn new(id: u16) -> Self {
        Self {
            _id: id,
            total_weight: 0,
            vruntime: 0,
            cpu_quota_ns: 0,
            cpu_used_ns: 0,
            nr_tasks: 0,
        }
    }

    /// Returns true if this group has exhausted its quota.
    fn is_throttled(&self) -> bool {
        group_is_throttled(self.cpu_quota_ns, self.cpu_used_ns)
    }
}

/// Per-task scheduling statistics (like Linux schedstat).
#[derive(Debug, Clone, Copy, Default)]
pub struct SchedStat {
    /// Total CPU time consumed (ns).
    pub cpu_time_ns: u64,
    /// Total time spent waiting in runqueue (ns).
    pub wait_time_ns: u64,
    /// Number of times this task was scheduled.
    pub run_count: u64,
    /// Number of involuntary preemptions.
    pub preempt_count: u64,
    /// Timestamp when task was last enqueued (for wait time calculation).
    pub last_enqueue_tick: u64,
    /// Timestamp when task was last scheduled to run.
    pub last_run_tick: u64,
}

pub struct CFS {
    /// Timeline: (vruntime, TaskId) -> (). Ordered by vruntime.
    timeline: BTreeMap<(u64, TaskId), ()>,

    /// Lookup: TaskId -> TaskMetadata
    task_metadata: BTreeMap<TaskId, TaskMetadata>,

    /// Task Storage
    tasks: BTreeMap<TaskId, Arc<IrqSafeMutex<KernelTask>>>,

    /// Monotonic minimum vruntime across the runqueue.
    min_vruntime: u64,

    /// CFS groups indexed by group_id.
    groups: BTreeMap<u16, CfsGroup>,

    /// Group-level timeline: (group_vruntime, group_id) -> ().
    group_timeline: BTreeMap<(u64, u16), ()>,

    /// Per-task scheduling statistics.
    schedstats: BTreeMap<TaskId, SchedStat>,

    /// Autogroups: session_id → Autogroup.
    autogroups: BTreeMap<usize, Autogroup>,

    /// Next autogroup CFS group id (starts high to avoid collision with manual groups).
    next_autogroup_id: u16,

    /// Global tick counter for timestamp calculations.
    tick_counter: u64,

    /// Elapsed CPU time budget for cgroup bandwidth period reset (microseconds).
    cgroup_period_elapsed_us: u64,
}

#[derive(Debug, Clone, Copy)]
struct TaskMetadata {
    vruntime: u64,
    weight: u64,
    group_id: u16,
}

impl Default for CFS {
    fn default() -> Self {
        Self::new()
    }
}

impl CFS {
    #[inline(always)]
    fn record_bootstrap_stage(stage: &'static str) {
        crate::kernel::debug_trace::record_optional("scheduler.cfs", stage, None, false);
    }

    #[inline(never)]
    pub fn new() -> Self {
        Self::record_bootstrap_stage("new_begin");
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs new begin\n");
        let timeline = BTreeMap::new();
        Self::record_bootstrap_stage("timeline_ready");
        let task_metadata = BTreeMap::new();
        Self::record_bootstrap_stage("task_metadata_ready");
        let tasks = BTreeMap::new();
        Self::record_bootstrap_stage("tasks_ready");
        let groups = BTreeMap::new();
        Self::record_bootstrap_stage("groups_ready");
        let group_timeline = BTreeMap::new();
        Self::record_bootstrap_stage("group_timeline_ready");
        let schedstats = BTreeMap::new();
        Self::record_bootstrap_stage("schedstats_ready");
        let autogroups = BTreeMap::new();
        Self::record_bootstrap_stage("autogroups_ready");
        let scheduler = Self {
            timeline,
            task_metadata,
            tasks,
            min_vruntime: 0,
            // Keep bootstrap construction allocation-free; the root group is
            // materialized lazily on first real scheduler use.
            groups,
            group_timeline,
            schedstats,
            autogroups,
            next_autogroup_id: 1000,
            tick_counter: 0,
            cgroup_period_elapsed_us: 0,
        };
        Self::record_bootstrap_stage("new_returned");
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs new returned\n");
        scheduler
    }

    fn ensure_root_group_initialized(&mut self) {
        if self.groups.contains_key(&0) {
            return;
        }
        self.groups.insert(0, CfsGroup::new(0));
        self.group_timeline.insert((0u64, 0u16), ());
    }

    fn bootstrap_pick_next_internal(&mut self) -> Option<TaskId> {
        let tid = self.timeline.iter().next().map(|(&(_, task_id), _)| task_id)?;
        if let Some(stat) = self.schedstats.get_mut(&tid) {
            let waited = self.tick_counter.saturating_sub(stat.last_enqueue_tick);
            stat.wait_time_ns = stat
                .wait_time_ns
                .saturating_add(waited * KernelConfig::time_slice());
            stat.run_count += 1;
            stat.last_run_tick = self.tick_counter;
        }
        Some(tid)
    }

    /// Create or get a CFS group. Returns false if max groups exceeded.
    pub fn create_group(&mut self, group_id: u16, cpu_quota_ns: u64) -> bool {
        if self.groups.len() >= MAX_CFS_GROUPS && !self.groups.contains_key(&group_id) {
            return false;
        }
        let group = self
            .groups
            .entry(group_id)
            .or_insert_with(|| CfsGroup::new(group_id));
        group.cpu_quota_ns = cpu_quota_ns;
        if !self.group_timeline.values().next().is_some()
            || !self
                .group_timeline
                .contains_key(&(group.vruntime, group_id))
        {
            self.group_timeline.insert((group.vruntime, group_id), ());
        }
        true
    }

    /// Reset all group CPU usage counters (called at period boundaries).
    pub fn reset_group_quotas(&mut self) {
        for group in self.groups.values_mut() {
            group.cpu_used_ns = 0;
        }
    }

    /// Convert a 0-255 priority (OS specific) to a CFS nice-weight.
    fn calculate_weight(priority: u8) -> u64 {
        calculate_weight(priority)
    }

    fn calc_delta_vruntime(delta_exec: u64, weight: u64) -> u64 {
        calc_delta_vruntime(delta_exec, weight)
    }

    /// Update min_vruntime to track the lowest vruntime smoothly.
    fn update_min_vruntime(&mut self) {
        if let Some((&(left_vruntime, _), _)) = self.timeline.iter().next() {
            if left_vruntime > self.min_vruntime {
                self.min_vruntime = left_vruntime;
            }
        }
    }

    /// Ensure a group exists (auto-create with default settings).
    fn ensure_group(&mut self, group_id: u16) {
        if !self.groups.contains_key(&group_id) {
            let mut g = CfsGroup::new(group_id);
            g.vruntime = self.min_vruntime;
            self.group_timeline.insert((g.vruntime, group_id), ());
            self.groups.insert(group_id, g);
        }
    }

    /// Advance group-level vruntime after a task in the group consumes CPU.
    fn charge_group(&mut self, group_id: u16, delta_exec: u64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            // Remove old position in group timeline
            self.group_timeline.remove(&(group.vruntime, group_id));
            // Charge group proportionally to its aggregate weight
            let group_weight = if group.total_weight > 0 {
                group.total_weight
            } else {
                NICE_0_LOAD
            };
            group.vruntime += Self::calc_delta_vruntime(delta_exec, group_weight);
            group.cpu_used_ns += delta_exec;
            // Re-insert with new vruntime
            self.group_timeline.insert((group.vruntime, group_id), ());
        }
    }

    /// Pick the best eligible (non-throttled) group.
    fn pick_best_group(&self) -> Option<u16> {
        for &(_, gid) in self.group_timeline.keys() {
            if let Some(group) = self.groups.get(&gid) {
                if group.nr_tasks > 0 && !group.is_throttled() {
                    return Some(gid);
                }
            }
        }
        None
    }

    // ── Schedstat ────────────────────────────────────────────

    /// Get scheduling statistics for a task.
    pub fn schedstat(&self, task_id: TaskId) -> Option<&SchedStat> {
        self.schedstats.get(&task_id)
    }

    /// Get scheduling statistics for all tasks.
    pub fn all_schedstats(&self) -> &BTreeMap<TaskId, SchedStat> {
        &self.schedstats
    }

}

pub(crate) fn priority_weight_for_contract(priority: u8) -> u64 {
    CFS::calculate_weight(priority)
}

impl Scheduler for CFS {
    type TaskItem = Arc<IrqSafeMutex<KernelTask>>;

    fn runqueue_len(&self) -> usize {
        self.tasks.len()
    }

    fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut Self::TaskItem> {
        self.tasks.get_mut(&task_id)
    }

    fn steal_task(&mut self) -> Option<Self::TaskItem> {
        // Steal the task with MAXIMUM vruntime (most CPU used, far right of the tree)
        let key = *self.timeline.keys().next_back()?;
        self.timeline.remove(&key);

        let (_, task_id) = key;
        if let Some(meta) = self.task_metadata.remove(&task_id) {
            // Update group accounting
            if let Some(group) = self.groups.get_mut(&meta.group_id) {
                group.total_weight = group.total_weight.saturating_sub(meta.weight);
                group.nr_tasks = group.nr_tasks.saturating_sub(1);
            }
        }
        self.update_min_vruntime();
        self.tasks.remove(&task_id)
    }

    fn init(&mut self) {
        // Reset state for a fresh start
        self.timeline.clear();
        self.task_metadata.clear();
        self.tasks.clear();
        self.min_vruntime = 0;
        self.groups.clear();
        self.group_timeline.clear();
        self.cgroup_period_elapsed_us = 0;
        // Re-create root group
        self.ensure_root_group_initialized();
    }

    fn add_task(&mut self, task_handle: Self::TaskItem) {
        self.ensure_root_group_initialized();
        let (weight, id, group_id) = {
            let task = task_handle.lock();
            (
                Self::calculate_weight(task.priority),
                task.id,
                task.cfs_group_id,
            )
        };

        // Ensure the group exists
        self.ensure_group(group_id);

        let vruntime = self.min_vruntime;

        self.task_metadata.insert(
            id,
            TaskMetadata {
                vruntime,
                weight,
                group_id,
            },
        );
        self.timeline.insert((vruntime, id), ());
        self.tasks.insert(id, task_handle);

        // Record schedstat enqueue time
        let stat = self.schedstats.entry(id).or_insert(SchedStat::default());
        stat.last_enqueue_tick = self.tick_counter;

        // Update group accounting
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.total_weight += weight;
            group.nr_tasks += 1;
        }

        self.update_min_vruntime();
    }

    fn remove_task(&mut self, task_id: TaskId) {
        if let Some(meta) = self.task_metadata.remove(&task_id) {
            self.timeline.remove(&(meta.vruntime, task_id));
            self.tasks.remove(&task_id);
            self.schedstats.remove(&task_id);
            // Update group accounting
            if let Some(group) = self.groups.get_mut(&meta.group_id) {
                group.total_weight = group.total_weight.saturating_sub(meta.weight);
                group.nr_tasks = group.nr_tasks.saturating_sub(1);
            }
            self.update_min_vruntime();
        }
    }

    fn remove_task_item(&mut self, task_id: TaskId) -> Option<Self::TaskItem> {
        if let Some(meta) = self.task_metadata.remove(&task_id) {
            self.timeline.remove(&(meta.vruntime, task_id));
            let task = self.tasks.remove(&task_id);
            // Update group accounting
            if let Some(group) = self.groups.get_mut(&meta.group_id) {
                group.total_weight = group.total_weight.saturating_sub(meta.weight);
                group.nr_tasks = group.nr_tasks.saturating_sub(1);
            }
            self.update_min_vruntime();
            task
        } else {
            None
        }
    }

    fn bootstrap_pick_next(&mut self) -> Option<TaskId> {
        self.bootstrap_pick_next_internal()
    }

    fn pick_next(&mut self) -> Option<TaskId> {
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs pick_next begin\n");
        self.update_min_vruntime();
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs pick_next after update_min_vruntime\n");

        if self.timeline.len() == 1 {
            #[cfg(all(target_arch = "x86_64", target_os = "none"))]
            crate::hal::x86_64::serial::write_raw(
                "[EARLY SERIAL] cfs pick_next singleton fast path\n",
            );
            let tid = self.bootstrap_pick_next_internal()?;
            #[cfg(all(target_arch = "x86_64", target_os = "none"))]
            crate::hal::x86_64::serial::write_raw(
                "[EARLY SERIAL] cfs pick_next singleton returned\n",
            );
            return Some(tid);
        }

        // Two-level scheduling: first pick the best group, then pick best task within.
        let picked = if let Some(best_gid) = self.pick_best_group() {
            #[cfg(all(target_arch = "x86_64", target_os = "none"))]
            crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs pick_next best group found\n");
            // Find the earliest-vruntime task belonging to this group
            let mut found = None;
            for (&(_, tid), _) in self.timeline.iter() {
                if let Some(meta) = self.task_metadata.get(&tid) {
                    if meta.group_id == best_gid {
                        found = Some(tid);
                        break;
                    }
                }
            }
            found
        } else {
            #[cfg(all(target_arch = "x86_64", target_os = "none"))]
            crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs pick_next no best group\n");
            None
        };

        let result = picked.or_else(|| {
            #[cfg(all(target_arch = "x86_64", target_os = "none"))]
            crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs pick_next fallback timeline begin\n");
            self.timeline
                .iter()
                .next()
                .map(|(&(_, task_id), _)| task_id)
        });
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs pick_next candidate ready\n");

        // Record schedstat: wait_time for picked task
        if let Some(tid) = result {
            #[cfg(all(target_arch = "x86_64", target_os = "none"))]
            crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs pick_next schedstat begin\n");
            if let Some(stat) = self.schedstats.get_mut(&tid) {
                #[cfg(all(target_arch = "x86_64", target_os = "none"))]
                crate::hal::x86_64::serial::write_raw(
                    "[EARLY SERIAL] cfs pick_next schedstat slot found\n",
                );
                let waited = self.tick_counter.saturating_sub(stat.last_enqueue_tick);
                #[cfg(all(target_arch = "x86_64", target_os = "none"))]
                crate::hal::x86_64::serial::write_raw(
                    "[EARLY SERIAL] cfs pick_next waited computed\n",
                );
                stat.wait_time_ns = stat
                    .wait_time_ns
                    .saturating_add(waited * KernelConfig::time_slice());
                #[cfg(all(target_arch = "x86_64", target_os = "none"))]
                crate::hal::x86_64::serial::write_raw(
                    "[EARLY SERIAL] cfs pick_next wait_time updated\n",
                );
                stat.run_count += 1;
                stat.last_run_tick = self.tick_counter;
                #[cfg(all(target_arch = "x86_64", target_os = "none"))]
                crate::hal::x86_64::serial::write_raw(
                    "[EARLY SERIAL] cfs pick_next schedstat updated\n",
                );
            } else {
                #[cfg(all(target_arch = "x86_64", target_os = "none"))]
                crate::hal::x86_64::serial::write_raw(
                    "[EARLY SERIAL] cfs pick_next schedstat slot missing\n",
                );
            }
        }

        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::x86_64::serial::write_raw("[EARLY SERIAL] cfs pick_next returned\n");
        result
    }

    fn tick(&mut self, current: TaskId) -> SchedulerAction {
        self.tick_counter += 1;
        let time_slice_ns = KernelConfig::time_slice();
        let min_granularity_ns = KernelConfig::cfs_min_granularity_ns();
        if let Some(meta) = self.task_metadata.get(&current).copied() {
            let delta_exec = time_slice_ns;
            let delta_vruntime = Self::calc_delta_vruntime(delta_exec, meta.weight);

            let new_vruntime = meta.vruntime + delta_vruntime;

            // Update schedstat
            if let Some(stat) = self.schedstats.get_mut(&current) {
                stat.cpu_time_ns = stat.cpu_time_ns.saturating_add(delta_exec);
            }

            // Re-insert task in timeline
            self.timeline.remove(&(meta.vruntime, current));
            self.task_metadata.insert(
                current,
                TaskMetadata {
                    vruntime: new_vruntime,
                    weight: meta.weight,
                    group_id: meta.group_id,
                },
            );
            self.timeline.insert((new_vruntime, current), ());

            // Charge the group
            self.charge_group(meta.group_id, delta_exec);

            // Charge the cgroup CPU controller (delta_exec is ns → convert to µs).
            // If the cgroup is throttled, force a reschedule immediately.
            {
                let cgroup_id = self
                    .tasks
                    .get(&current)
                    .map(|t| t.lock().cgroup_id)
                    .unwrap_or(crate::kernel::cgroups::ROOT_CGROUP_ID);
                let us = core::cmp::max(1, delta_exec / 1_000); // nanoseconds → microseconds
                if !crate::kernel::cgroups::cgroup_charge_cpu(cgroup_id, us) {
                    if let Some(stat) = self.schedstats.get_mut(&current) {
                        stat.preempt_count += 1;
                    }
                    self.update_min_vruntime();
                    return SchedulerAction::Reschedule;
                }

                self.cgroup_period_elapsed_us = self.cgroup_period_elapsed_us.saturating_add(us);
                if self.cgroup_period_elapsed_us >= CGROUP_CPU_PERIOD_US {
                    crate::kernel::cgroups::cgroup_reset_all_periods();
                    self.cgroup_period_elapsed_us %= CGROUP_CPU_PERIOD_US;
                }
            }

            self.update_min_vruntime();

            // Preemption: check if a different group or task should run
            if let Some(best_gid) = self.pick_best_group() {
                if best_gid != meta.group_id {
                    // Record involuntary preemption in schedstat
                    if let Some(stat) = self.schedstats.get_mut(&current) {
                        stat.preempt_count += 1;
                    }
                    return SchedulerAction::Reschedule;
                }
            }

            // Within-group preemption check
            if let Some((&(left_vruntime, left_task), _)) = self.timeline.iter().next() {
                if left_task != current && new_vruntime > left_vruntime {
                    if should_preempt_current(new_vruntime, left_vruntime, min_granularity_ns) {
                        if let Some(stat) = self.schedstats.get_mut(&current) {
                            stat.preempt_count += 1;
                        }
                        return SchedulerAction::Reschedule;
                    }
                }
            }
        }

        SchedulerAction::Continue
    }
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    fn make_task(id: usize, priority: u8) -> Arc<IrqSafeMutex<KernelTask>> {
        Arc::new(IrqSafeMutex::new(KernelTask::new(
            TaskId(id),
            priority,
            0,
            0,
            0x2000,
            0,
            0x1000,
        )))
    }

    #[test_case]
    fn singleton_pick_next_returns_only_task_and_updates_schedstats() {
        let mut sched = CFS::new();
        sched.init();

        let task = make_task(1, 128);
        sched.add_task(task);
        sched.tick_counter = 7;

        let picked = sched.pick_next();
        assert_eq!(picked, Some(TaskId(1)));

        let stat = sched.schedstat(TaskId(1)).copied().unwrap_or_default();
        assert_eq!(stat.run_count, 1);
        assert_eq!(stat.last_run_tick, 7);
    }

    #[test_case]
    fn singleton_pick_next_survives_missing_root_group_bootstrap_state() {
        let mut sched = CFS::new();
        let task = make_task(7, 100);
        sched.add_task(task);

        assert_eq!(sched.pick_next(), Some(TaskId(7)));
        assert_eq!(sched.runqueue_len(), 1);
    }
}

