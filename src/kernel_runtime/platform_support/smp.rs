pub(crate) fn init_smp_runtime(enabled: bool) {
    if enabled {
        hypercore::hal::HAL::init_smp();
        #[cfg(target_arch = "aarch64")]
        {
            let st = hypercore::hal::aarch64::smp::boot_stats();
            hypercore::klog_info!(
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
        hypercore::klog_info!("SMP initialization enabled");
    } else {
        hypercore::klog_warn!("SMP initialization disabled by config");
    }
}
