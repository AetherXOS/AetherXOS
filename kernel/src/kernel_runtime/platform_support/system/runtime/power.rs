use super::super::current_virtualization_log_snapshot;

pub(crate) fn log_power_baseline() {
    let pwr = aethercore::kernel::power::stats();
    let virt = current_virtualization_log_snapshot();
    aethercore::klog_info!(
        "Power baseline: idle_calls={} c1={} c2={} c3={} pstate_switches={} current_pstate={:?} p_override_active={} p_override_set={} p_override_clear={} c_override_active={} c_override_set={} c_override_clear={} acpi_loaded={} fadt_rev={} guard_hits={} rq_clamps={} failsafe={} override_rejects_no_acpi={} virt_exec_profile={} virt_lane={} virt_mode={} virt_governor={} latency_bias={} energy_bias={}",
        pwr.idle_calls,
        pwr.c1_entries,
        pwr.c2_entries,
        pwr.c3_entries,
        pwr.pstate_switches,
        pwr.current_pstate,
        pwr.policy_override_active,
        pwr.policy_override_set_calls,
        pwr.policy_override_clear_calls,
        pwr.cstate_override_active,
        pwr.cstate_override_set_calls,
        pwr.cstate_override_clear_calls,
        pwr.acpi_profile_loaded,
        pwr.acpi_fadt_revision,
        pwr.policy_guard_hits,
        pwr.runqueue_clamp_events,
        pwr.failsafe_idle_fallbacks,
        pwr.override_rejects_no_acpi,
        virt.execution_profile,
        virt.scheduler_lane,
        virt.selected_mode,
        virt.governor_class,
        virt.latency_bias,
        virt.energy_bias
    );
}
