pub(crate) fn init_virtualization_bootstrap(enabled: bool) {
    let virt = hypercore::hal::virt::detect_caps();
    #[cfg(feature = "telemetry")]
    hypercore::klog_info!(
        "Virtualization caps: vmx={} svm={} hypervisor={}",
        virt.vmx,
        virt.svm,
        virt.hypervisor_present
    );

    if enabled {
        let enabled_state = hypercore::hal::virt::try_enable_hardware_virtualization();
        #[cfg(feature = "telemetry")]
        hypercore::klog_info!(
            "Virtualization enabled: vmx={} vmxon={} svm={}",
            enabled_state.vmx_enabled,
            enabled_state.vmxon_active,
            enabled_state.svm_enabled
        );

        let launch_ctx_active = hypercore::hal::virt::initialize_launch_context();
        #[cfg(feature = "telemetry")]
        hypercore::klog_info!(
            "Virtualization launch context initialized: {}",
            launch_ctx_active
        );
    } else {
        #[cfg(feature = "telemetry")]
        hypercore::klog_info!("Virtualization enable path disabled by config");
    }

    let virt_status = hypercore::hal::virt::status();
    #[cfg(feature = "telemetry")]
    hypercore::klog_info!(
        "Virtualization readiness: ready={} blocker={} vmx={} svm={} hypervisor={} vmx_enabled={} vmxon={} svm_enabled={} vmcs_ready={} vmcb_ready={} vmx_lifecycle={} svm_lifecycle={} prep_attempts={} prep_ok={} prep_fail={}",
        virt_status.vm_launch_ready,
        virt_status.blocker,
        virt_status.caps.vmx,
        virt_status.caps.svm,
        virt_status.caps.hypervisor_present,
        virt_status.enabled.vmx_enabled,
        virt_status.enabled.vmxon_active,
        virt_status.enabled.svm_enabled,
        virt_status.vmx_vmcs_ready,
        virt_status.svm_vmcb_ready,
        virt_status.vmx_lifecycle,
        virt_status.svm_lifecycle,
        virt_status.prep_attempts,
        virt_status.prep_success,
        virt_status.prep_failures
    );
}
