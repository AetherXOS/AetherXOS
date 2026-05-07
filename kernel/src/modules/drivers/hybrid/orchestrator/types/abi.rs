use super::super::super::liblinux::{LibLinuxConformanceRisk, LibLinuxSemanticClass, LinuxSyscall};
use super::super::super::LinuxIoRequestKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridUserspaceAbiContractRow {
    pub syscall: LinuxSyscall,
    pub io_kind: LinuxIoRequestKind,
    pub semantic_class: LibLinuxSemanticClass,
    pub zero_copy_eligible: bool,
    pub supported: bool,
    pub behavior_depth: HybridUserspaceAbiBehaviorDepth,
    pub risk: LibLinuxConformanceRisk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridUserspaceAbiContractMatrix {
    pub rows: Vec<HybridUserspaceAbiContractRow>,
    pub supported_ratio: u8,
    pub behavior_depth_ratio: u8,
    pub data_path_rows: usize,
    pub control_path_rows: usize,
    pub memory_map_rows: usize,
    pub full_depth_rows: usize,
    pub partial_depth_rows: usize,
    pub stub_depth_rows: usize,
    pub high_risk_ops: usize,
    pub release_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridUserspaceAbiBehaviorDepth {
    Full,
    Partial,
    Stub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridUserspaceAbiTailPressureLevel {
    Insufficient,
    Stable,
    Observe,
    Warn,
    Block,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridUserspaceAbiReport {
    pub readiness_score: u8,
    pub confidence_score: u8,
    pub telemetry_shape_score: u8,
    pub tail_pressure_level: HybridUserspaceAbiTailPressureLevel,
    pub telemetry_samples: usize,
    pub compile_linux_compat: bool,
    pub compile_vfs: bool,
    pub boundary_allows_compat: bool,
    pub expose_linux_compat_surface: bool,
    pub attack_surface_budget: u8,
    pub attack_surface_target: u8,
    pub critical_blockers: usize,
    pub high_blockers: usize,
    pub medium_blockers: usize,
    pub effective_surface_enabled: bool,
    pub contract_matrix: HybridUserspaceAbiContractMatrix,
    pub blockers: Vec<&'static str>,
    pub next_action: &'static str,
    pub release_ready: bool,
}
