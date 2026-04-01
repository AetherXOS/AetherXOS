use crate::interfaces::TaskId;

use super::{CFS, TaskMetadata};

/// Autogroup — groups tasks by session/tty for interactive fairness.
/// Each autogroup gets its own CFS group with equal weight, preventing
/// a compile job with 100 threads from starving a single interactive shell.
#[derive(Debug, Clone)]
pub struct Autogroup {
    /// Session id that this autogroup represents.
    pub session_id: usize,
    /// CFS group_id assigned to this autogroup.
    pub cfs_group_id: u16,
    /// Nice value for the autogroup (adjusts group weight).
    pub nice: i8,
    /// Number of tasks currently in this autogroup.
    pub task_count: usize,
}

impl CFS {
    /// Create or join an autogroup for a session.
    /// If the session already has an autogroup, returns its group_id.
    /// Otherwise, creates a new CFS group for the session.
    pub fn autogroup_join(&mut self, session_id: usize, task_id: TaskId) -> u16 {
        if let Some(ag) = self.autogroups.get_mut(&session_id) {
            ag.task_count += 1;
            return ag.cfs_group_id;
        }
        // Create new autogroup
        let gid = self.next_autogroup_id;
        self.next_autogroup_id = self.next_autogroup_id.wrapping_add(1);
        self.create_group(gid, 0); // 0 = unlimited quota
        self.autogroups.insert(
            session_id,
            Autogroup {
                session_id,
                cfs_group_id: gid,
                nice: 0,
                task_count: 1,
            },
        );
        // Update task's group if it's in the runqueue
        if let Some(meta) = self.task_metadata.get(&task_id).copied() {
            self.timeline.remove(&(meta.vruntime, task_id));
            let new_meta = TaskMetadata {
                group_id: gid,
                ..meta
            };
            self.task_metadata.insert(task_id, new_meta);
            self.timeline.insert((new_meta.vruntime, task_id), ());
            // Update group counters
            self.decrement_group_tasks(meta.group_id);
            self.increment_group_tasks(gid, meta.weight);
        }
        gid
    }

    /// Leave an autogroup (on task exit or session change).
    pub fn autogroup_leave(&mut self, session_id: usize) {
        if let Some(ag) = self.autogroups.get_mut(&session_id) {
            ag.task_count = ag.task_count.saturating_sub(1);
            if ag.task_count == 0 {
                let gid = ag.cfs_group_id;
                self.autogroups.remove(&session_id);
                // Clean up the CFS group
                self.group_timeline
                    .remove(&(self.groups.get(&gid).map(|g| g.vruntime).unwrap_or(0), gid));
                self.groups.remove(&gid);
            }
        }
    }

    /// Set nice value for an autogroup.
    pub fn autogroup_set_nice(&mut self, session_id: usize, nice: i8) {
        if let Some(ag) = self.autogroups.get_mut(&session_id) {
            ag.nice = nice;
        }
    }

    /// Get autogroup info for a session.
    pub fn autogroup_info(&self, session_id: usize) -> Option<&Autogroup> {
        self.autogroups.get(&session_id)
    }

    fn increment_group_tasks(&mut self, group_id: u16, weight: u64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.nr_tasks += 1;
            group.total_weight += weight;
        }
    }

    fn decrement_group_tasks(&mut self, group_id: u16) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.nr_tasks = group.nr_tasks.saturating_sub(1);
        }
    }
}
