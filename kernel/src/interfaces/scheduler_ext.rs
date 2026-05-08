/// Scheduler extension interfaces.

use crate::interfaces::{KernelResult, TaskId};

/// Priority level for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityLevel {
    RealtimeHigh = 0,
    RealtimeNormal = 1,
    RealtimeLow = 2,
    Interactive = 3,
    Normal = 4,
    Low = 5,
    Batch = 6, // Added Batch
    Idle = 7,
}

impl PriorityLevel {
    pub fn priority_value(&self) -> u8 {
        match self {
            Self::RealtimeHigh => 100,
            Self::RealtimeNormal => 80,
            Self::RealtimeLow => 60,
            Self::Interactive => 40,
            Self::Normal => 30,
            Self::Low => 20,
            Self::Batch => 10,
            Self::Idle => 0,
        }
    }

    pub fn is_realtime(&self) -> bool {
        matches!(self, Self::RealtimeHigh | Self::RealtimeNormal | Self::RealtimeLow)
    }
}

/// Scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedulingPolicy {
    CFS,
    RealTimeFifo,
    RealTimeRoundRobin,
    Deadline,
    Batch,
}

/// Group identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SchedulingGroupId(pub u32);

/// Trait for priority-based scheduling
pub trait SchedulerWithPriority {
    fn set_task_priority(&self, task_id: TaskId, priority: PriorityLevel) -> KernelResult<()>;
    fn get_task_priority(&self, task_id: TaskId) -> KernelResult<PriorityLevel>;
    fn set_scheduling_policy(&self, task_id: TaskId, policy: SchedulingPolicy) -> KernelResult<()>;
    fn get_scheduling_policy(&self, task_id: TaskId) -> KernelResult<SchedulingPolicy>;
    fn priority_levels(&self) -> &'static [PriorityLevel];
}

/// Trait for scheduling groups
pub trait SchedulerWithGroups {
    fn create_group(&self, cpu_quota_ns: u64, cpu_period_ns: u64) -> KernelResult<SchedulingGroupId>;
    fn delete_group(&self, group_id: SchedulingGroupId) -> KernelResult<()>;
    fn add_task_to_group(&self, task_id: TaskId, group_id: SchedulingGroupId) -> KernelResult<()>;
    fn remove_task_from_group(&self, task_id: TaskId) -> KernelResult<()>;
    fn set_cpu_affinity(&self, task_id: TaskId, mask: u64) -> KernelResult<()>;
    fn get_cpu_affinity(&self, task_id: TaskId) -> KernelResult<u64>;
    fn set_cpu_quota(&self, group_id: SchedulingGroupId, quota_ns: u64) -> KernelResult<()>;
}

/// Real-time deadline
#[derive(Debug, Clone, Copy)]
pub struct RealTimeDeadline {
    pub period_ns: u64,
    pub runtime_ns: u64,
}

/// Trait for real-time scheduling
pub trait SchedulerRealTime {
    fn set_realtime_deadline(&self, task_id: TaskId, deadline: RealTimeDeadline) -> KernelResult<()>;
    fn get_realtime_deadline(&self, task_id: TaskId) -> KernelResult<RealTimeDeadline>;
    fn can_admit_realtime(&self, runtime_ns: u64, period_ns: u64) -> bool;
    fn realtime_load(&self) -> f64;
    fn realtime_capacity_remaining(&self) -> f64;
}
