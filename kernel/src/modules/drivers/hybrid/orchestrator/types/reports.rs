use super::{BackendPreference, HybridRequestKind, HybridRequestFamily, HybridGapSeverity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridBackendSupport {
    pub backend: BackendPreference,
    pub score: u8,
    pub supported: bool,
    pub degraded: bool,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSupportReport {
    pub request_kind: HybridRequestKind,
    pub entries: Vec<HybridBackendSupport>,
    pub recommended: BackendPreference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridPerformanceTier {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridSecurityPosture {
    Isolated,
    Mediated,
    CompatibilityRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridRuntimeConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridBackendAssessment {
    pub backend: BackendPreference,
    pub supported: bool,
    pub confidence: HybridRuntimeConfidence,
    pub performance: HybridPerformanceTier,
    pub security: HybridSecurityPosture,
    pub risk: &'static str,
    pub notes: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridRuntimeAssessmentReport {
    pub request_kind: HybridRequestKind,
    pub recommended: BackendPreference,
    pub assessments: Vec<HybridBackendAssessment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridBackendFleetStatus {
    pub backend: BackendPreference,
    pub coverage_score: u8,
    pub performance_score: u8,
    pub security_score: u8,
    pub supported_request_kinds: usize,
    pub unsupported_request_kinds: usize,
    pub high_risk_paths: usize,
    pub ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridFamilyFleetStatus {
    pub family: HybridRequestFamily,
    pub coverage_score: u8,
    pub performance_score: u8,
    pub security_score: u8,
    pub supported_request_kinds: usize,
    pub unsupported_request_kinds: usize,
    pub high_risk_paths: usize,
    pub ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridFleetReport {
    pub backends: Vec<HybridBackendFleetStatus>,
    pub families: Vec<HybridFamilyFleetStatus>,
    pub most_ready_backend: BackendPreference,
    pub least_ready_backend: BackendPreference,
    pub overall_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridMaturityDimension {
    TelemetryCoverage,
    TailLatency,
    ThreatModelCoverage,
    CertificationMatrix,
    FailoverConsistency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridMaturityFinding {
    pub dimension: HybridMaturityDimension,
    pub score: u8,
    pub gap: HybridGapSeverity,
    pub summary: &'static str,
    pub remediation: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridMaturityReport {
    pub findings: Vec<HybridMaturityFinding>,
    pub overall_score: u8,
    pub production_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridReadinessGap {
    pub request_kind: Option<HybridRequestKind>,
    pub severity: HybridGapSeverity,
    pub issue: &'static str,
    pub remediation: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridReadinessReport {
    pub coverage: super::audits::HybridCoverageAudit,
    pub userspace_abi: super::abi::HybridUserspaceAbiReport,
    pub virtualization: super::virtualization::HybridVirtualizationReadinessReport,
    pub gaps: Vec<HybridReadinessGap>,
    pub release_ready: bool,
}
