mod network;
mod nvme;

use self::network::{
    apply_aggressive_low_latency_network_policy, apply_low_latency_network_policy,
    apply_throughput_network_policy,
};
use self::nvme::{
    apply_balanced_nvme_queue_profile, apply_low_latency_nvme_queue_profile,
    apply_throughput_nvme_queue_profile,
};
use super::preset::PRESET_APPLY_CALLS;
use super::*;

pub fn apply_runtime_policy_preset() {
    PRESET_APPLY_CALLS.fetch_add(1, Ordering::Relaxed);
    let preset = runtime_policy_preset();

    match preset {
        CoreRuntimePolicyPreset::Interactive => {
            apply_low_latency_network_policy();
            apply_balanced_nvme_queue_profile();
            crate::config::KernelConfig::set_vfs_enable_buffered_io(Some(true));
            crate::kernel::rt_preemption::set_force_threshold_override_ticks(None);
        }
        CoreRuntimePolicyPreset::Server => {
            apply_throughput_network_policy();
            apply_throughput_nvme_queue_profile();
            crate::config::KernelConfig::set_vfs_enable_buffered_io(Some(true));
            crate::kernel::rt_preemption::set_force_threshold_override_ticks(None);
        }
        CoreRuntimePolicyPreset::Realtime => {
            apply_aggressive_low_latency_network_policy();
            apply_low_latency_nvme_queue_profile();
            crate::config::KernelConfig::set_vfs_enable_buffered_io(Some(false));
            crate::kernel::rt_preemption::set_force_threshold_override_ticks(Some(1));
        }
    }
}
