pub const NICE_0_LOAD: u64 = 1024;

pub fn calculate_weight(_priority: u8) -> u64 {
    // Basic priority to weight mapping (similar to Linux)
    // For now, just use a simple linear mapping or constant
    NICE_0_LOAD
}

pub fn calc_delta_vruntime(delta_exec: u64, weight: u64) -> u64 {
    if weight == 0 { return delta_exec; }
    (delta_exec * NICE_0_LOAD) / weight
}

pub fn migration_cost(
    from_core: Option<u32>,
    to_core: Option<u32>,
    from_llc: Option<u16>,
    to_llc: Option<u16>,
    from_node: Option<u8>,
    to_node: Option<u8>,
) -> u64 {
    if from_core == to_core {
        return 0;
    }
    if from_llc == to_llc {
        return 10_000; // 10us
    }
    if from_node == to_node {
        return 100_000; // 100us
    }
    500_000 // 500us default
}

pub fn group_is_throttled(quota_ns: u64, used_ns: u64) -> bool {
    if quota_ns == 0 {
        return false;
    }
    used_ns >= quota_ns
}

pub fn should_preempt_current(new_vruntime: u64, current_vruntime: u64, min_granularity: u64) -> bool {
    new_vruntime + min_granularity < current_vruntime
}
