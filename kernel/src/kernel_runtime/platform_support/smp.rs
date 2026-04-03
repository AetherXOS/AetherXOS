pub(crate) fn init_smp_runtime(enabled: bool) {
    if enabled {
        aethercore::hal::HAL::init_smp();
        #[cfg(target_arch = "aarch64")]
        {
            let st = aethercore::hal::smp::boot_stats();
            aethercore::klog_info!(
                "SMP boot stats: hvc={}/{} smc={}/{} failures={} timeouts={} aps_ready={}",
                st.hvc_success,
                st.hvc_attempts,
                st.smc_success,
                st.smc_attempts,
                st.boot_failures,
                st.boot_timeouts,
                st.aps_ready
            );
        }
        aethercore::klog_info!("SMP initialization enabled");
    } else {
        aethercore::klog_warn!("SMP initialization disabled by config");
    }
}
