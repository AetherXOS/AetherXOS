use crate::modules::drivers::hybrid::orchestrator::HybridUserspaceAbiReport;
use crate::modules::drivers::hybrid::orchestrator::types::abi::{HybridUserspaceAbiContractMatrix, HybridUserspaceAbiTailPressureLevel};
use crate::modules::drivers::hybrid::liblinux::LibLinuxTelemetryStore;

fn stub_report() -> HybridUserspaceAbiReport {
    HybridUserspaceAbiReport {
        readiness_score: 0,
        confidence_score: 0,
        telemetry_shape_score: 0,
        tail_pressure_level: HybridUserspaceAbiTailPressureLevel::Insufficient,
        telemetry_samples: 0,
        compile_linux_compat: false,
        compile_vfs: false,
        boundary_allows_compat: false,
        expose_linux_compat_surface: false,
        attack_surface_budget: 0,
        attack_surface_target: 0,
        critical_blockers: 0,
        high_blockers: 0,
        medium_blockers: 0,
        effective_surface_enabled: false,
        contract_matrix: HybridUserspaceAbiContractMatrix {
            rows: alloc::vec::Vec::new(),
            supported_ratio: 0,
            behavior_depth_ratio: 0,
            data_path_rows: 0,
            control_path_rows: 0,
            memory_map_rows: 0,
            full_depth_rows: 0,
            partial_depth_rows: 0,
            stub_depth_rows: 0,
            high_risk_ops: 0,
            release_ready: false,
        },
        blockers: alloc::vec::Vec::new(),
        next_action: "",
        release_ready: false,
    }
}

pub fn userspace_abi_report() -> HybridUserspaceAbiReport {
    stub_report()
}

pub fn userspace_abi_report_with_telemetry(_l: Option<&LibLinuxTelemetryStore>) -> HybridUserspaceAbiReport {
    stub_report()
}
