use super::*;

#[derive(Clone)]
pub(super) struct ParentTaskSnapshot {
    pub priority: u8,
    pub deadline: u64,
    pub burst_time: u64,
    pub cfs_group_id: u16,
    pub cgroup_id: u64,
    pub uid: u32,
    pub gid: u32,
    pub security_ctx: crate::interfaces::security::SecurityContext,
    pub resource_limits: crate::interfaces::security::ResourceLimits,
    pub cpu_affinity_mask: u64,
    pub preferred_cpu: crate::interfaces::task::CpuId,
    pub signal_mask: u64,
    pub signal_stack: Option<crate::interfaces::task::SignalStack>,
    #[cfg(feature = "ring_protection")]
    pub user_tls_base: u64,
}

pub(super) fn snapshot_parent_task(parent_pid: ProcessId) -> Option<ParentTaskSnapshot> {
    let current_tid = unsafe { crate::kernel::cpu_local::CpuLocal::get() }.current_task_id();

    let current_task = get_task(current_tid).and_then(|task_arc| {
        let task = task_arc.lock();
        if task.process_id == Some(parent_pid) {
            Some(task.clone())
        } else {
            None
        }
    });

    let fallback_task = || {
        let process = get_process(parent_pid)?;
        let primary_tid = *process.threads.lock().first()?;
        let task_arc = get_task(primary_tid)?;
        let snapshot = task_arc.lock().clone();
        Some(snapshot)
    };

    let parent_task = current_task.or_else(fallback_task)?;
    Some(ParentTaskSnapshot {
        priority: parent_task.priority,
        deadline: parent_task.deadline,
        burst_time: parent_task.burst_time,
        cfs_group_id: parent_task.cfs_group_id,
        cgroup_id: parent_task.cgroup_id,
        uid: parent_task.uid,
        gid: parent_task.gid,
        security_ctx: parent_task.security_ctx,
        resource_limits: parent_task.resource_limits,
        cpu_affinity_mask: parent_task.cpu_affinity_mask,
        preferred_cpu: parent_task.preferred_cpu,
        signal_mask: parent_task.signal_mask,
        signal_stack: parent_task.signal_stack,
        #[cfg(feature = "ring_protection")]
        user_tls_base: parent_task.user_tls_base,
    })
}
