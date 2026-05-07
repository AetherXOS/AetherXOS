use super::cfs_support::group_is_throttled;

#[derive(Debug)]
pub struct CfsGroup {
    /// Group identifier (0 = root group).
    pub _id: u16,
    /// Aggregate weight of all tasks in this group.
    pub total_weight: u64,
    /// Group-level vruntime (advances proportionally to group weight).
    pub vruntime: u64,
    /// CPU quota in nanoseconds per period (0 = unlimited).
    pub cpu_quota_ns: u64,
    /// CPU consumed in current period.
    pub cpu_used_ns: u64,
    /// Number of tasks in this group.
    pub nr_tasks: usize,
}

impl CfsGroup {
    pub fn new(id: u16) -> Self {
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
    pub fn is_throttled(&self) -> bool {
        group_is_throttled(self.cpu_quota_ns, self.cpu_used_ns)
    }
}
