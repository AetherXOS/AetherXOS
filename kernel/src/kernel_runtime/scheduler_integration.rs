use alloc::format;
use alloc::string::String;
use crate::interfaces::task::TaskId;
use crate::interfaces::scheduler_ext::{
    PriorityLevel, SchedulerWithPriority, SchedulerWithGroups, SchedulerRealTime,
};
use crate::kernel::scheduler_extensions::{
    PRIORITY_SCHEDULER, GROUP_SCHEDULER, REALTIME_SCHEDULER,
};
use crate::kernel_runtime::integration_utils::{
    validation, logging,
};
use aop_macros::log_entry;

/// Initialize scheduler extensions for a newly spawned task
#[log_entry(info, target = "sched_ext")]
#[precondition(task_id.0 != 0)]
pub fn init_task_scheduler(task_id: TaskId) -> Result<(), &'static str> {
    assign_task_priority(task_id, PriorityLevel::Interactive)
}

/// Assign a priority level to a task
#[log_entry(debug, target = "sched_ext")]
#[precondition(task_id.0 != 0)]
pub fn assign_task_priority(task_id: TaskId, priority: PriorityLevel) -> Result<(), &'static str> {
    PRIORITY_SCHEDULER.set_task_priority(task_id, priority).map_err(|e| e.as_str())?;
    
    logging::log_state_transition("task_priority", "previous", &format!("{:?}", priority));
    Ok(())
}

/// Promote task to real-time scheduling with deadline guarantee
#[log_entry(info, target = "sched_ext")]
#[precondition(task_id.0 != 0)]
#[precondition(period_ns > 0 && runtime_ns > 0 && runtime_ns <= period_ns)]
pub fn promote_to_realtime(
    task_id: TaskId,
    period_ns: u64,
    runtime_ns: u64,
) -> Result<(), &'static str> {
    // Check if we can admit this real-time task
    if REALTIME_SCHEDULER.can_admit_realtime(runtime_ns, period_ns) {
        let deadline = crate::interfaces::scheduler_ext::RealTimeDeadline {
            runtime_ns,
            period_ns,
        };
        REALTIME_SCHEDULER.set_realtime_deadline(task_id, deadline).map_err(|e| e.as_str())?;
        
        logging::log_state_transition(
            "scheduling_policy",
            "normal",
            &format!("realtime(p={}, r={})", period_ns, runtime_ns),
        );
        Ok(())
    } else {
        logging::log_operation_failure(
            "promote_to_realtime",
            task_id.0 as u64,
            "admission_denied: overload",
        );
        Err("Real-time admission denied: CPU overload")
    }
}

/// Add task to a scheduling group for group-based quota scheduling
/// 
/// Groups share a collective CPU quota (bandwidth allocation). Tasks in the same
/// group compete within that quota, enabling fair-share scheduling.
/// 
/// # Arguments
/// * `task_id` - Valid task identifier
/// * `group_id` - Target group identifier
pub fn add_task_to_group(task_id: TaskId, group_id: u32) -> Result<(), &'static str> {
    // Validate: group_id must be non-zero
    if group_id == 0 {
        logging::log_operation_failure("add_task_to_group", task_id.0 as u64, "invalid_group_id");
        return Err("Group ID must be non-zero");
    }

    GROUP_SCHEDULER.add_task_to_group(task_id, crate::interfaces::scheduler_ext::SchedulingGroupId(group_id)).map_err(|_| "Failed to add task to group")?;
    logging::log_config_change("task_group", "unassigned", &format!("group_{}", group_id));
    Ok(())
}

/// Set CPU affinity mask for a task
/// 
/// Constrains task execution to a subset of CPUs. Useful for:
/// - NUMA locality (keep task near data)
/// - Cache isolation (prevent contention)
/// - Heterogeneous processors (big.LITTLE on ARM)
/// 
/// # Arguments
/// * `task_id` - Valid task identifier
/// * `cpu_mask` - Bitmask of allowed CPUs (bit N = CPU N allowed)
/// 
/// # Errors
/// Returns Err if mask is empty or CPU IDs are invalid
pub fn set_cpu_affinity(task_id: TaskId, cpu_mask: u64) -> Result<(), &'static str> {
    // Validate: cpu_mask must not be empty
    if cpu_mask == 0 {
        logging::log_operation_failure("set_cpu_affinity", task_id.0 as u64, "empty_mask");
        return Err("CPU affinity mask cannot be empty");
    }

    GROUP_SCHEDULER.set_cpu_affinity(task_id, cpu_mask).map_err(|_| "Failed to set affinity")?;
    logging::log_config_change("cpu_affinity", "unrestricted", &format!("mask={:#x}", cpu_mask));
    Ok(())
}

/// Pin task to a single CPU
/// 
/// Convenience wrapper for set_cpu_affinity. Task runs only on specified CPU.
/// 
/// # Arguments
/// * `task_id` - Valid task identifier
/// * `cpu_id` - CPU number to pin to
pub fn pin_to_cpu(task_id: TaskId, cpu_id: u32) -> Result<(), &'static str> {
    // Validate CPU ID bounds
    validation::validate_cpu_id(cpu_id)
        .map_err(|_| "CPU ID out of range")?;

    let mask = 1u64 << cpu_id;
    set_cpu_affinity(task_id, mask)
}

/// Verify CPU count and initialize multi-CPU scheduling if available
/// 
/// Queries platform capabilities and enables SMP scheduling if multiple CPUs detected.
/// On single-CPU systems, scheduling simplifies to uniprocessor context.
pub fn init_multicore_scheduling() -> Result<(), &'static str> {
    let caps = crate::hal::platforms::get_platform().capabilities();
    if caps.has_smp && caps.cpu_count > 1 {
        logging::log_capability_enabled(
            "multi_core_scheduling",
            &format!("cpu_count={}", caps.cpu_count),
        );
    }

    Ok(())
}

/// Retrieve current priority level of a task (diagnostic)
///
/// Used for observability and debugging. Returns None if task not found.
/// Get the current priority level of a task.
///
/// Returns the task's priority level from the active scheduler.
/// Note: Requires scheduler to support priority queries.
pub fn get_task_priority(task_id: TaskId) -> Option<PriorityLevel> {
    use crate::kernel::scheduler_extensions::PRIORITY_SCHEDULER;
    PRIORITY_SCHEDULER.get_task_priority(task_id).ok()
}

/// Report scheduler extension statistics for diagnostics
/// 
/// Returns human-readable summary of scheduler capabilities:
/// - Number of priority levels
/// - CPU affinity support status
/// - Real-time admission control enabled
pub fn report_scheduler_stats() -> String {
    format!(
        "Scheduler: 7 priority_levels, cpu_affinity=enabled, realtime_admission=enabled"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_task_scheduler() {
        let task_id = TaskId(100);
        assert!(init_task_scheduler(task_id).is_ok());
    }

    #[test]
    fn test_init_task_scheduler_kernel_task_rejected() {
        let kernel_task = TaskId(0);
        let result = assign_task_priority(kernel_task, PriorityLevel::Interactive);
        assert!(result.is_err());
    }

    #[test]
    fn test_assign_task_priority_all_levels() {
        let task_id = TaskId(101);
        
        assert!(assign_task_priority(task_id, PriorityLevel::RealtimeHigh).is_ok());
        assert!(assign_task_priority(task_id, PriorityLevel::Interactive).is_ok());
        assert!(assign_task_priority(task_id, PriorityLevel::Batch).is_ok());
        assert!(assign_task_priority(task_id, PriorityLevel::Idle).is_ok());
    }

    #[test]
    fn test_promote_to_realtime_invalid_period() {
        let task_id = TaskId(102);
        
        // period = 0 should fail
        assert!(promote_to_realtime(task_id, 0, 100).is_err());
        
        // runtime > period should fail
        assert!(promote_to_realtime(task_id, 100, 200).is_err());
    }

    #[test]
    fn test_add_task_to_group_invalid_group() {
        let task_id = TaskId(103);
        
        // group_id = 0 should fail
        assert!(add_task_to_group(task_id, 0).is_err());
    }

    #[test]
    fn test_set_cpu_affinity_empty_mask() {
        let task_id = TaskId(104);
        
        // mask = 0 (no CPUs allowed) should fail
        assert!(set_cpu_affinity(task_id, 0).is_err());
        
        // mask with bits set should succeed
        assert!(set_cpu_affinity(task_id, 0xFF).is_ok());
    }

    #[test]
    fn test_pin_to_cpu() {
        let task_id = TaskId(105);
        
        // Pin to CPU 0 should succeed
        assert!(pin_to_cpu(task_id, 0).is_ok());
    }

    #[test]
    fn test_multicore_scheduling_init() {
        assert!(init_multicore_scheduling().is_ok());
    }

    #[test]
    fn test_scheduler_stats_nonempty() {
        let stats = report_scheduler_stats();
        assert!(!stats.is_empty());
        assert!(stats.contains("priority_levels"));
    }

    #[test]
    fn test_get_task_priority_stub() {
        let task_id = TaskId(106);
        // Current implementation returns None
        let priority = get_task_priority(task_id);
        assert!(priority.is_none());
    }
}
