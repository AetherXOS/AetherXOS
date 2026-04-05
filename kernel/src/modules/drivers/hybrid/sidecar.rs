mod transport;
mod vm;
mod wire;
mod telemetry;

pub use transport::*;
pub use vm::*;
pub use wire::*;
pub use telemetry::*;

pub mod bootstrap;
pub use bootstrap::{
    SideCarBootstrapPhase, SideCarBootstrapState, SideCarBootstrapSummary, SideCarRetryPolicy,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::drivers::hybrid::linux::{build_network_plan, LinuxShimDeviceKind};
    use crate::modules::drivers::hybrid::LinuxBridgePayload;
    use crate::modules::drivers::{DriverTransportKind, LinuxBridgeMessage, LinuxBridgeMessageKind};

    #[test_case]
    fn sidecar_plan_for_gpu_uses_large_rings() {
        let cfg = SideCarVmConfig::new(2, 2, 256 * 1024 * 1024);
        let plan = SideCarVmPlan::for_linux_device(cfg, LinuxShimDeviceKind::Gpu, 77);
        assert!(plan.control_ring_depth >= 256);
        assert!(plan.data_ring_depth >= 1024);
    }

    #[test_case]
    fn sidecar_plan_adapts_to_workload_pressure() {
        let cfg = SideCarVmConfig::new(5, 4, 512 * 1024 * 1024);
        let low = SideCarVmPlan::for_linux_device_with_workload(
            cfg,
            LinuxShimDeviceKind::Storage,
            41,
            SideCarWorkloadProfile::from_resource_lengths(0x80, 0x800),
        );
        let high = SideCarVmPlan::for_linux_device_with_workload(
            cfg,
            LinuxShimDeviceKind::Storage,
            41,
            SideCarWorkloadProfile::from_resource_lengths(0x400, 0x8000),
        );

        assert!(high.control_ring_depth > low.control_ring_depth);
        assert!(high.data_ring_depth > low.data_ring_depth);
        assert!(high.irq_route.coalescing_budget >= low.irq_route.coalescing_budget);
    }

    #[test_case]
    fn telemetry_feedback_increases_pressure_under_stress() {
        let base = SideCarWorkloadProfile::from_resource_lengths(0x100, 0x1000);
        let summary = SideCarTelemetrySummary {
            sample_count: 8,
            avg_queue_fill_pct: 88,
            avg_retry_count: 3,
            max_p99_latency_us: 3_500,
            max_irq_burst: 96,
        };

        let tuned = apply_telemetry_feedback(base, summary);
        assert!(tuned.dma_pressure_hint > base.dma_pressure_hint);
        assert!(tuned.iova_bytes > base.iova_bytes);
        assert!(tuned.mmio_bytes > base.mmio_bytes);
    }

    #[test_case]
    fn telemetry_store_produces_tuned_profile_for_device() {
        let mut store = SideCarTelemetryStore::new(8);
        store.record(
            LinuxShimDeviceKind::Network,
            SideCarTelemetrySample::new(90, 2, 2_200, 80),
        );
        store.record(
            LinuxShimDeviceKind::Network,
            SideCarTelemetrySample::new(86, 3, 2_600, 72),
        );

        let base = SideCarWorkloadProfile::from_resource_lengths(0x100, 0x1000);
        let tuned = store.tuned_workload_profile(LinuxShimDeviceKind::Network, base);

        assert!(tuned.dma_pressure_hint > base.dma_pressure_hint);
        assert!(tuned.iova_bytes >= base.iova_bytes);
    }

    #[test_case]
    fn telemetry_store_evicts_stale_device_when_limit_reached() {
        let mut store = SideCarTelemetryStore::new_with_limits(2, 4);
        store.record(
            LinuxShimDeviceKind::Network,
            SideCarTelemetrySample::new(70, 1, 1500, 32),
        );
        store.record(
            LinuxShimDeviceKind::Storage,
            SideCarTelemetrySample::new(65, 1, 1400, 24),
        );
        store.record(
            LinuxShimDeviceKind::Gpu,
            SideCarTelemetrySample::new(88, 2, 2600, 96),
        );

        assert_eq!(store.device_count(), 2);
        assert!(store.summary_for(LinuxShimDeviceKind::Network).is_none());
        assert!(store.summary_for(LinuxShimDeviceKind::Gpu).is_some());
    }

    #[test_case]
    fn telemetry_store_snapshot_roundtrip_preserves_recent_data() {
        let mut original = SideCarTelemetryStore::new_with_limits(4, 6);
        original.record(
            LinuxShimDeviceKind::Network,
            SideCarTelemetrySample::new(84, 2, 2200, 64),
        );
        original.record(
            LinuxShimDeviceKind::Network,
            SideCarTelemetrySample::new(88, 3, 2600, 72),
        );

        let snapshot = original.snapshot();
        let restored = SideCarTelemetryStore::from_snapshot(snapshot, 4, 6);

        let before = original
            .summary_for(LinuxShimDeviceKind::Network)
            .expect("original summary should exist");
        let after = restored
            .summary_for(LinuxShimDeviceKind::Network)
            .expect("restored summary should exist");

        assert_eq!(before.avg_queue_fill_pct, after.avg_queue_fill_pct);
        assert_eq!(before.max_p99_latency_us, after.max_p99_latency_us);
    }

    #[test_case]
    fn telemetry_snapshot_bytes_roundtrip() {
        let mut store = SideCarTelemetryStore::new_with_limits(3, 4);
        store.record(
            LinuxShimDeviceKind::Network,
            SideCarTelemetrySample::new(80, 2, 2100, 60),
        );
        store.record(
            LinuxShimDeviceKind::Storage,
            SideCarTelemetrySample::new(55, 1, 1300, 22),
        );

        let snapshot = store.snapshot();
        let bytes = snapshot.to_bytes();
        let restored_snapshot = SideCarTelemetrySnapshot::from_bytes(&bytes)
            .expect("snapshot bytes should decode");
        let restored = SideCarTelemetryStore::from_snapshot(restored_snapshot, 3, 4);

        assert!(restored.summary_for(LinuxShimDeviceKind::Network).is_some());
        assert!(restored.summary_for(LinuxShimDeviceKind::Storage).is_some());
    }

    #[test_case]
    fn vm_plan_constructor_uses_telemetry_feedback_path() {
        let cfg = SideCarVmConfig::new(15, 2, 256 * 1024 * 1024);
        let base_workload = SideCarWorkloadProfile::from_resource_lengths(0x100, 0x1000);

        let baseline = SideCarVmPlan::for_linux_device_with_telemetry(
            cfg,
            LinuxShimDeviceKind::Network,
            40,
            base_workload,
            None,
        );

        let mut telemetry = SideCarTelemetryStore::new(8);
        telemetry.record(
            LinuxShimDeviceKind::Network,
            SideCarTelemetrySample::new(92, 3, 3000, 100),
        );

        let tuned = SideCarVmPlan::for_linux_device_with_telemetry(
            cfg,
            LinuxShimDeviceKind::Network,
            40,
            base_workload,
            Some(&telemetry),
        );

        assert!(tuned.data_ring_depth > baseline.data_ring_depth);
        assert!(tuned.irq_route.coalescing_budget >= baseline.irq_route.coalescing_budget);
    }

    #[test_case]
    fn sidecar_plan_can_derive_from_linux_plan() {
        let linux_plan = build_network_plan(
            DriverTransportKind::SideCarVm,
            0x1000,
            0x100,
            0x2000,
            0x2000,
            32,
        );
        let cfg = SideCarVmConfig::new(1, 1, 128 * 1024 * 1024);
        let plan = SideCarVmPlan::from_linux_resource_plan(cfg, &linux_plan);
        assert_eq!(plan.control_ring_depth, linux_plan.control_queue_depth);
        assert_eq!(plan.data_ring_depth, linux_plan.data_queue_depth);
    }

    #[test_case]
    fn wire_header_roundtrip() {
        let header = build_wire_notify(3, 99, VirtioQueueSelector::Tx, 256);
        let encoded = encode_wire_header(header);
        let decoded = decode_wire_header(&encoded).expect("wire header should decode");
        assert_eq!(decoded, header);
    }

    #[test_case]
    fn payload_roundtrip_queue_notify() {
        let payload = SideCarPayload::QueueNotify {
            queue: VirtioQueueSelector::Rx,
            desc_count: 7,
            bytes: 1500,
        };
        let encoded = encode_payload(&payload);
        let decoded = decode_payload(&encoded).expect("payload should decode");
        assert_eq!(decoded, payload);
    }

    #[test_case]
    fn in_memory_transport_records_wire_frames() {
        let mut transport = InMemorySideCarTransport::new();
        transport
            .notify_queue(VirtioQueueSelector::Tx)
            .expect("notify should succeed");
        let frame = transport.pop_wire_frame().expect("frame should exist");
        assert_eq!(frame.0.opcode, SideCarOpcode::NotifyQueue);
    }

    #[test_case]
    fn bootstrap_state_advances_from_bridge_completion() {
        let mut state = SideCarBootstrapState::new(200, SideCarRetryPolicy::conservative());
        let completion = LinuxBridgeMessage::new(
            LinuxBridgeMessageKind::QueryStatus,
            200,
            LinuxBridgePayload::Completion(super::super::DriverCompletion::ok(200, 0)),
        );

        assert!(state.apply_bridge_message(&completion, 0));
        assert_eq!(state.phase, SideCarBootstrapPhase::ControlNotify);
    }
}
