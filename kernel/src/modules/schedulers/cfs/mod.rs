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

use crate::interfaces::{KernelTask, TaskId};
use crate::kernel::sync::IrqSafeMutex;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

#[path = "cfs_support.rs"]
pub mod cfs_support;
pub mod autogroup;
pub mod topology;
pub mod group;
pub mod stats;
pub mod metadata;
mod scheduler_impl;

pub use autogroup::Autogroup;
pub use topology::{CpuTopology, CpuTopologyEntry, SchedDomainLevel};
pub use group::CfsGroup;
pub use stats::SchedStat;
pub use metadata::TaskMetadata;

use cfs_support::{
    calc_delta_vruntime, calculate_weight,
    NICE_0_LOAD,
};

pub const CGROUP_CPU_PERIOD_US: u64 = 100_000;
/// Maximum number of CFS groups supported.
pub const MAX_CFS_GROUPS: usize = 64;

pub struct CFS {
    /// Timeline: (vruntime, TaskId) -> (). Ordered by vruntime.
    pub(crate) timeline: BTreeMap<(u64, TaskId), ()>,

    /// Lookup: TaskId -> TaskMetadata
    pub(crate) task_metadata: BTreeMap<TaskId, TaskMetadata>,

    /// Task Storage
    pub(crate) tasks: BTreeMap<TaskId, Arc<IrqSafeMutex<KernelTask>>>,

    /// Monotonic minimum vruntime across the runqueue.
    pub(crate) min_vruntime: u64,

    /// CFS groups indexed by group_id.
    pub(crate) groups: BTreeMap<u16, CfsGroup>,

    /// Group-level timeline: (group_vruntime, group_id) -> ().
    pub(crate) group_timeline: BTreeMap<(u64, u16), ()>,

    /// Per-task scheduling statistics.
    pub(crate) schedstats: BTreeMap<TaskId, SchedStat>,

    /// Autogroups: session_id → Autogroup.
    pub(crate) autogroups: BTreeMap<usize, Autogroup>,

    /// Next autogroup CFS group id (starts high to avoid collision with manual groups).
    pub(crate) next_autogroup_id: u16,

    /// Global tick counter for timestamp calculations.
    pub(crate) tick_counter: u64,

    /// Elapsed CPU time budget for cgroup bandwidth period reset (microseconds).
    pub(crate) cgroup_period_elapsed_us: u64,
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
        crate::hal::serial::write_raw("[EARLY SERIAL] cfs new begin\n");
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
        crate::hal::serial::write_raw("[EARLY SERIAL] cfs new returned\n");
        scheduler
    }

    pub(crate) fn ensure_root_group_initialized(&mut self) {
        if self.groups.contains_key(&0) {
            return;
        }
        self.groups.insert(0, CfsGroup::new(0));
        self.group_timeline.insert((0u64, 0u16), ());
    }

    pub(crate) fn bootstrap_pick_next_internal(&mut self) -> Option<TaskId> {
        use crate::config::KernelConfig;
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
    pub(crate) fn calculate_weight(priority: u8) -> u64 {
        calculate_weight(priority)
    }

    pub(crate) fn calc_delta_vruntime(delta_exec: u64, weight: u64) -> u64 {
        calc_delta_vruntime(delta_exec, weight)
    }

    /// Update min_vruntime to track the lowest vruntime smoothly.
    pub(crate) fn update_min_vruntime(&mut self) {
        if let Some((&(left_vruntime, _), _)) = self.timeline.iter().next() {
            if left_vruntime > self.min_vruntime {
                self.min_vruntime = left_vruntime;
            }
        }
    }

    /// Ensure a group exists (auto-create with default settings).
    pub(crate) fn ensure_group(&mut self, group_id: u16) {
        if !self.groups.contains_key(&group_id) {
            let mut g = CfsGroup::new(group_id);
            g.vruntime = self.min_vruntime;
            self.group_timeline.insert((g.vruntime, group_id), ());
            self.groups.insert(group_id, g);
        }
    }

    /// Advance group-level vruntime after a task in the group consumes CPU.
    pub(crate) fn charge_group(&mut self, group_id: u16, delta_exec: u64) {
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
    pub(crate) fn pick_best_group(&self) -> Option<u16> {
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

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;
    use crate::interfaces::Scheduler;

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
