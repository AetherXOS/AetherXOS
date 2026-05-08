// --- PHASE 5: ADVANCED SCHEDULER EXTENSIONS ---
// Priority levels, scheduling policies, real-time guarantees

use crate::core::log;
use alloc::format;
use crate::interfaces::scheduler_ext::{
    PriorityLevel, SchedulingPolicy, SchedulerRealTime, SchedulerWithGroups,
    SchedulerWithPriority, RealTimeDeadline, SchedulingGroupId,
};
use crate::interfaces::TaskId;
use alloc::collections::BTreeMap;
use crate::kernel::sync::IrqSafeMutex;

/// Task priority assignment
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskPriority {
    pub task_id: u32,
    pub priority: PriorityLevel,
    pub policy: SchedulingPolicy,
}

/// CPU affinity mask (which CPUs a task can run on)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuAffinity {
    pub mask: u64,
}

impl CpuAffinity {
    /// Create affinity for a single CPU
    pub fn single_cpu(cpu_id: u32) -> Self {
        Self {
            mask: 1u64 << cpu_id,
        }
    }

    /// Create affinity for all CPUs
    pub fn all_cpus() -> Self {
        Self { mask: u64::MAX }
    }

    /// Check if a CPU is in the affinity mask
    pub fn contains_cpu(&self, cpu_id: u32) -> bool {
        if cpu_id >= 64 {
            return false;
        }
        (self.mask & (1u64 << cpu_id)) != 0
    }
}

/// Scheduling group for group scheduling
#[derive(Debug, Clone, Copy)]
pub struct SchedulingGroup {
    pub id: SchedulingGroupId,
    pub cpu_quota_ns: u64,    // Nanoseconds per scheduling period
    pub cpu_period_ns: u64,   // Length of scheduling period
}

/// Concrete implementation of SchedulerWithPriority
pub struct PriorityScheduler {
    /// Task → priority mapping
    priorities: IrqSafeMutex<BTreeMap<TaskId, PriorityLevel>>,

    /// Task → policy mapping
    policies: IrqSafeMutex<BTreeMap<TaskId, SchedulingPolicy>>,
}

impl PriorityScheduler {
    /// Create a new priority scheduler
    pub const fn new() -> Self {
        Self {
            priorities: IrqSafeMutex::new(BTreeMap::new()),
            policies: IrqSafeMutex::new(BTreeMap::new()),
        }
    }
}

impl SchedulerWithPriority for PriorityScheduler {
    /// Set task priority level
    fn set_task_priority(&self, task_id: TaskId, priority: PriorityLevel) -> crate::interfaces::KernelResult<()> {
        if task_id.0 == 0 {
            return Err(crate::interfaces::KernelError::PermissionDenied);
        }

        self.priorities.lock().insert(task_id, priority);
        log::debug(&format!(
            "Task {} priority set to {:?}",
            task_id, priority
        ));
        Ok(())
    }

    /// Get current task priority level
    fn get_task_priority(&self, task_id: TaskId) -> crate::interfaces::KernelResult<PriorityLevel> {
        self.priorities.lock().get(&task_id).copied().ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// Set scheduling policy for a task
    fn set_scheduling_policy(
        &self,
        task_id: TaskId,
        policy: SchedulingPolicy,
    ) -> crate::interfaces::KernelResult<()> {
        match policy {
            SchedulingPolicy::RealTimeFifo | SchedulingPolicy::RealTimeRoundRobin => {
                // Real-time tasks need higher priority
                let priority = self
                    .get_task_priority(task_id)
                    .unwrap_or(PriorityLevel::Interactive);
                if priority > PriorityLevel::RealtimeLow {
                    return Err(crate::interfaces::KernelError::PermissionDenied);
                }
            }
            _ => {}
        }

        self.policies.lock().insert(task_id, policy);
        log::debug(&format!(
            "Task {} scheduling policy set to {:?}",
            task_id, policy
        ));
        Ok(())
    }

    /// Get scheduling policy for a task
    fn get_scheduling_policy(&self, task_id: TaskId) -> crate::interfaces::KernelResult<SchedulingPolicy> {
        self.policies.lock().get(&task_id).copied().ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// Get available priority levels
    fn priority_levels(&self) -> &'static [PriorityLevel] {
        &[
            PriorityLevel::RealtimeHigh,
            PriorityLevel::RealtimeNormal,
            PriorityLevel::RealtimeLow,
            PriorityLevel::Interactive,
            PriorityLevel::Normal,
            PriorityLevel::Low,
            PriorityLevel::Batch,
            PriorityLevel::Idle,
        ]
    }
}

/// Concrete implementation of SchedulerWithGroups
pub struct GroupScheduler {
    /// Scheduling groups
    groups: IrqSafeMutex<BTreeMap<SchedulingGroupId, SchedulingGroup>>,

    /// Task → group mapping
    task_groups: IrqSafeMutex<BTreeMap<TaskId, SchedulingGroupId>>,

    /// CPU affinity per task
    affinity: IrqSafeMutex<BTreeMap<TaskId, CpuAffinity>>,

    /// Next group ID
    next_group_id: IrqSafeMutex<u32>,
}

impl GroupScheduler {
    /// Create a new group scheduler
    pub const fn new() -> Self {
        Self {
            groups: IrqSafeMutex::new(BTreeMap::new()),
            task_groups: IrqSafeMutex::new(BTreeMap::new()),
            affinity: IrqSafeMutex::new(BTreeMap::new()),
            next_group_id: IrqSafeMutex::new(1),
        }
    }
}

impl SchedulerWithGroups for GroupScheduler {
    /// Create a new scheduling group
    fn create_group(
        &self,
        cpu_quota_ns: u64,
        cpu_period_ns: u64,
    ) -> crate::interfaces::KernelResult<SchedulingGroupId> {
        if cpu_quota_ns > cpu_period_ns {
            return Err(crate::interfaces::KernelError::InvalidInput);
        }

        let id = SchedulingGroupId(*self.next_group_id.lock());
        *self.next_group_id.lock() += 1;

        let group = SchedulingGroup {
            id,
            cpu_quota_ns,
            cpu_period_ns,
        };

        self.groups.lock().insert(id, group);
        log::debug(&format!("Created scheduling group {:?}", id));
        Ok(id)
    }

    /// Delete a scheduling group
    fn delete_group(&self, group_id: SchedulingGroupId) -> crate::interfaces::KernelResult<()> {
        // Check if group has tasks
        let has_tasks = self
            .task_groups
            .lock()
            .values()
            .any(|&g| g == group_id);

        if has_tasks {
            return Err(crate::interfaces::KernelError::Busy);
        }

        self.groups.lock().remove(&group_id);
        log::debug(&format!("Deleted scheduling group {:?}", group_id));
        Ok(())
    }

    /// Add task to scheduling group
    fn add_task_to_group(
        &self,
        task_id: TaskId,
        group_id: SchedulingGroupId,
    ) -> crate::interfaces::KernelResult<()> {
        if !self.groups.lock().contains_key(&group_id) {
            return Err(crate::interfaces::KernelError::NotFound);
        }

        self.task_groups.lock().insert(task_id, group_id);
        log::debug(&format!("Task {} added to group {:?}", task_id, group_id));
        Ok(())
    }

    fn remove_task_from_group(&self, task_id: TaskId) -> crate::interfaces::KernelResult<()> {
        self.task_groups.lock().remove(&task_id);
        Ok(())
    }

    /// Set CPU affinity for a task
    fn set_cpu_affinity(&self, task_id: TaskId, affinity: u64) -> crate::interfaces::KernelResult<()> {
        self.affinity.lock().insert(
            task_id,
            CpuAffinity { mask: affinity },
        );
        log::debug(&format!(
            "Task {} CPU affinity set to {:#x}",
            task_id, affinity
        ));
        Ok(())
    }

    /// Get CPU affinity for a task
    fn get_cpu_affinity(&self, task_id: TaskId) -> crate::interfaces::KernelResult<u64> {
        self.affinity
            .lock()
            .get(&task_id)
            .map(|a| a.mask)
            .ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// Set per-group CPU quota
    fn set_cpu_quota(&self, group_id: SchedulingGroupId, quota_ns: u64) -> crate::interfaces::KernelResult<()> {
        if let Some(group) = self.groups.lock().get_mut(&group_id) {
            if quota_ns > group.cpu_period_ns {
                return Err(crate::interfaces::KernelError::InvalidInput);
            }
            group.cpu_quota_ns = quota_ns;
            log::debug(&format!("Group {:?} quota set to {} ns", group_id, quota_ns));
            Ok(())
        } else {
            Err(crate::interfaces::KernelError::NotFound)
        }
    }
}

/// Concrete implementation of SchedulerRealTime
pub struct RealTimeScheduler {
    /// Real-time deadlines per task
    deadlines: IrqSafeMutex<BTreeMap<TaskId, RealTimeDeadline>>,

    /// Current real-time load (sum of CPU fractions)
    realtime_load: IrqSafeMutex<f64>,
}

impl RealTimeScheduler {
    /// Create a new real-time scheduler
    pub const fn new() -> Self {
        Self {
            deadlines: IrqSafeMutex::new(BTreeMap::new()),
            realtime_load: IrqSafeMutex::new(0.0),
        }
    }
}

impl SchedulerRealTime for RealTimeScheduler {
    /// Set real-time deadline for a task
    fn set_realtime_deadline(
        &self,
        task_id: TaskId,
        deadline: RealTimeDeadline,
    ) -> crate::interfaces::KernelResult<()> {
        // Check admission: ensure we don't overload the CPU
        let cpu_fraction = deadline.runtime_ns as f64 / deadline.period_ns as f64;
        let new_load = *self.realtime_load.lock() + cpu_fraction;

        if new_load > 1.0 {
            return Err(crate::interfaces::KernelError::LimitExceeded);
        }

        self.deadlines.lock().insert(task_id, deadline);
        *self.realtime_load.lock() = new_load;

        log::info(&format!(
            "Real-time task {} admitted: {:?}ns / {:?}ns (load: {:.1}%)",
            task_id, deadline.runtime_ns, deadline.period_ns, new_load * 100.0
        ));
        Ok(())
    }

    /// Get real-time deadline for a task
    fn get_realtime_deadline(&self, task_id: TaskId) -> crate::interfaces::KernelResult<RealTimeDeadline> {
        self.deadlines.lock().get(&task_id).copied().ok_or(crate::interfaces::KernelError::NotFound)
    }

    /// Check if a real-time task can be admitted
    fn can_admit_realtime(&self, runtime_ns: u64, period_ns: u64) -> bool {
        let cpu_fraction = runtime_ns as f64 / period_ns as f64;
        let new_load = *self.realtime_load.lock() + cpu_fraction;
        new_load <= 1.0
    }

    /// Get current real-time load
    fn realtime_load(&self) -> f64 {
        *self.realtime_load.lock()
    }

    /// Get real-time capacity remaining
    fn realtime_capacity_remaining(&self) -> f64 {
        1.0 - self.realtime_load()
    }
}

// Global scheduler instances
pub static PRIORITY_SCHEDULER: PriorityScheduler = PriorityScheduler::new();
pub static GROUP_SCHEDULER: GroupScheduler = GroupScheduler::new();
pub static REALTIME_SCHEDULER: RealTimeScheduler = RealTimeScheduler::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_scheduler_creation() {
        let sched = PriorityScheduler::new();
        assert_eq!(sched.priority_levels().len(), 8);
    }

    #[test]
    fn test_set_task_priority() {
        let sched = PriorityScheduler::new();
        let tid = TaskId(1);
        assert!(sched.set_task_priority(tid, PriorityLevel::Interactive).is_ok());
        assert_eq!(
            sched.get_task_priority(tid).unwrap(),
            PriorityLevel::Interactive
        );
    }

    #[test]
    fn test_cannot_change_idle_priority() {
        let sched = PriorityScheduler::new();
        assert!(sched
            .set_task_priority(TaskId(0), PriorityLevel::Batch)
            .is_err());
    }

    #[test]
    fn test_cpu_affinity_single() {
        let aff = CpuAffinity::single_cpu(2);
        assert!(aff.contains_cpu(2));
        assert!(!aff.contains_cpu(1));
        assert!(!aff.contains_cpu(3));
    }

    #[test]
    fn test_cpu_affinity_all() {
        let aff = CpuAffinity::all_cpus();
        for i in 0..64 {
            assert!(aff.contains_cpu(i));
        }
    }

    #[test]
    fn test_group_scheduler_creation() {
        let sched = GroupScheduler::new();
        let group = sched.create_group(5_000_000, 10_000_000);
        assert!(group.is_ok());
    }

    #[test]
    fn test_group_scheduler_quota_validation() {
        let sched = GroupScheduler::new();
        // Quota > period should fail
        assert!(sched.create_group(15_000_000, 10_000_000).is_err());
    }

    #[test]
    fn test_add_task_to_group() {
        let sched = GroupScheduler::new();
        let group = sched.create_group(5_000_000, 10_000_000).unwrap();
        assert!(sched.add_task_to_group(TaskId(1), group).is_ok());
    }

    #[test]
    fn test_set_cpu_affinity() {
        let sched = GroupScheduler::new();
        let tid = TaskId(1);
        assert!(sched.set_cpu_affinity(tid, 0x0F).is_ok());
        assert_eq!(sched.get_cpu_affinity(tid).unwrap(), 0x0F);
    }

    #[test]
    fn test_realtime_scheduler_admission() {
        let sched = RealTimeScheduler::new();
        let deadline = RealTimeDeadline {
            period_ns: 10_000_000,  // 10 ms
            runtime_ns: 5_000_000,  // 5 ms (50% load)
        };
        assert!(sched.set_realtime_deadline(TaskId(1), deadline).is_ok());
        assert!(sched.can_admit_realtime(5_000_000, 10_000_000));
    }

    #[test]
    fn test_realtime_scheduler_overload() {
        let sched = RealTimeScheduler::new();

        // First task: 50% load
        let deadline1 = RealTimeDeadline {
            period_ns: 10_000_000,
            runtime_ns: 5_000_000,
        };
        sched.set_realtime_deadline(TaskId(1), deadline1).ok();

        // Second task: 60% load (total 110% - should fail)
        let deadline2 = RealTimeDeadline {
            period_ns: 10_000_000,
            runtime_ns: 6_000_000,
        };
        assert!(sched.set_realtime_deadline(TaskId(2), deadline2).is_err());
    }

    #[test]
    fn test_realtime_load_calculation() {
        let sched = RealTimeScheduler::new();

        assert_eq!(sched.realtime_load(), 0.0);
        assert_eq!(sched.realtime_capacity_remaining(), 1.0);

        let deadline = RealTimeDeadline {
            period_ns: 10_000_000,
            runtime_ns: 3_000_000,
        };
        sched.set_realtime_deadline(TaskId(1), deadline).ok();

        assert!((sched.realtime_load() - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_scheduling_policy_validation() {
        let sched = PriorityScheduler::new();
        let tid = TaskId(1);

        // Set low priority
        sched.set_task_priority(tid, PriorityLevel::Batch).ok();

        // Try to set real-time policy - should fail (since Batch > RealtimeLow in our enum order)
        // Wait, RealtimeHigh=0, ..., Idle=7. So Batch(6) > RealtimeLow(2).
        assert!(sched
            .set_scheduling_policy(tid, SchedulingPolicy::RealTimeFifo)
            .is_err());
    }
}
