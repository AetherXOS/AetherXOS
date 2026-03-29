pub(crate) fn log_security_telemetry() {
    let dma_status = hypercore::modules::security::dma_protection_status();
    let sec = hypercore::modules::security::telemetry();
    hypercore::klog_info!(
        "DMA protection: active={} backend={} hw_mode={} devices={} mappings={}",
        hypercore::modules::security::is_dma_protection_active(),
        dma_status.backend,
        dma_status.hardware_mode,
        dma_status.protected_devices,
        dma_status.mapped_regions
    );
    hypercore::klog_info!(
        "Security telemetry: profile={:?} dma_active={} acl_grant={} acl_revoke={} acl_check={} acl_hits={} cap_mint={} cap_revoke={} cap_check={} cap_hits={}",
        sec.profile,
        sec.dma_active,
        sec.acl_grant_calls,
        sec.acl_revoke_calls,
        sec.acl_check_calls,
        sec.acl_check_hits,
        sec.cap_mint_calls,
        sec.cap_revoke_calls,
        sec.cap_check_calls,
        sec.cap_check_hits
    );
}

pub(crate) fn log_ipc_telemetry() {
    #[cfg(feature = "ipc_message_passing")]
    let ipc_mp = hypercore::modules::ipc::message_passing::stats();
    #[cfg(feature = "ipc_zero_copy")]
    let ipc_zc = hypercore::modules::ipc::zero_copy::stats();
    #[cfg(feature = "ipc_signal_only")]
    let ipc_sig = hypercore::modules::ipc::signal_only::stats();
    #[cfg(feature = "ipc_futex")]
    let ipc_futex = hypercore::modules::ipc::futex::stats();

    hypercore::klog_info!("IPC telemetry summary:");
    #[cfg(feature = "ipc_message_passing")]
    hypercore::klog_info!(
        "  MessagePassing: create={} send={} drop_oversize={} drop_backpressure={} recv={} hits={} trunc={}",
        ipc_mp.channel_create_calls,
        ipc_mp.send_calls,
        ipc_mp.send_drops_oversize,
        ipc_mp.send_drops_backpressure,
        ipc_mp.receive_calls,
        ipc_mp.receive_hits,
        ipc_mp.receive_truncated
    );
    #[cfg(feature = "ipc_zero_copy")]
    hypercore::klog_info!(
        "  ZeroCopy: set={} send={} drop_oversize={} recv={} hits={} small_buf={}",
        ipc_zc.set_buffer_calls,
        ipc_zc.send_calls,
        ipc_zc.send_drops_oversize,
        ipc_zc.receive_calls,
        ipc_zc.receive_hits,
        ipc_zc.receive_small_buffer
    );
    #[cfg(feature = "ipc_signal_only")]
    hypercore::klog_info!(
        "  SignalOnly: send={} recv={} hits={}",
        ipc_sig.send_calls,
        ipc_sig.receive_calls,
        ipc_sig.receive_hits
    );
    #[cfg(feature = "ipc_futex")]
    hypercore::klog_info!(
        "  Futex: wait={} enqueue={} mismatch={} wake={} woken={} send={} invalid={} recv={} hits={} small_buf={} event_drops={}",
        ipc_futex.wait_calls,
        ipc_futex.wait_enqueued,
        ipc_futex.wait_value_mismatch,
        ipc_futex.wake_calls,
        ipc_futex.wake_woken,
        ipc_futex.send_calls,
        ipc_futex.send_invalid_control,
        ipc_futex.receive_calls,
        ipc_futex.receive_hits,
        ipc_futex.receive_small_buffer,
        ipc_futex.wake_event_drops
    );
}
