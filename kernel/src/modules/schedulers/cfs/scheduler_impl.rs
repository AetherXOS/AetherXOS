use crate::interfaces::{KernelTask, Scheduler, SchedulerAction, TaskId};
use crate::core::log;
use crate::kernel::sync::IrqSafeMutex;
use crate::config::KernelConfig;
use alloc::sync::Arc;
use super::*;

macro_rules! early_serial_log {
    ($msg:expr) => {
        #[cfg(all(target_arch = "x86_64", target_os = "none"))]
        crate::hal::serial::write_raw(concat!("[EARLY SERIAL] ", $msg, "\n"));
    };
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
        log::trace("CFS: pick_next starting");
        self.update_min_vruntime();
        log::trace("CFS: vruntime updated");

        if self.timeline.len() == 1 {
            log::trace("CFS: singleton fast path");
            let tid = self.bootstrap_pick_next_internal()?;
            log::trace("CFS: singleton task found");
            return Some(tid);
        }

        // Two-level scheduling: first pick the best group, then pick best task within.
        let picked = if let Some(best_gid) = self.pick_best_group() {
            log::trace("CFS: best group found");
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
            log::trace("CFS: no best group");
            None
        };

        let result = picked.or_else(|| {
            log::trace("CFS: fallback timeline lookup");
            self.timeline
                .iter()
                .next()
                .map(|(&(_, task_id), _)| task_id)
        });
        log::trace("CFS: candidate ready for execution");

        // Record schedstat: wait_time for picked task
        if let Some(tid) = result {
            early_serial_log!("cfs pick_next schedstat begin");
            if let Some(stat) = self.schedstats.get_mut(&tid) {
                early_serial_log!("cfs pick_next schedstat slot found");
                let waited = self.tick_counter.saturating_sub(stat.last_enqueue_tick);
                early_serial_log!("cfs pick_next waited computed");
                stat.wait_time_ns = stat
                    .wait_time_ns
                    .saturating_add(waited * KernelConfig::time_slice());
                early_serial_log!("cfs pick_next wait_time updated");
                stat.run_count += 1;
                stat.last_run_tick = self.tick_counter;
                early_serial_log!("cfs pick_next schedstat updated");
            } else {
                early_serial_log!("cfs pick_next schedstat slot missing");
            }
        }

        early_serial_log!("cfs pick_next returned");
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

            // Update the task's own telemetry if available
            if let Some(task_handle) = self.tasks.get(&current) {
                let mut task = task_handle.lock();
                task.time_consumed = task.time_consumed.saturating_add(delta_exec);
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
                    if cfs_support::should_preempt_current(new_vruntime, left_vruntime, min_granularity_ns) {
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
