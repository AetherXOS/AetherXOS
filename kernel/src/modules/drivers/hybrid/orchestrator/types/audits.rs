use super::{BackendPreference, HybridRequestKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridCoverageRow {
    pub request_kind: HybridRequestKind,
    pub supported_backends: Vec<BackendPreference>,
    pub recommended: BackendPreference,
    pub coverage_score: u8,
    pub has_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridCoverageAudit {
    pub rows: Vec<HybridCoverageRow>,
    pub overall_score: u8,
    pub all_requests_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridFeatureKind {
    Mmio,
    Dma,
    Irq,
    SharedMemory,
    ControlQueue,
    Reset,
    Hotplug,
    PowerManagement,
    Snapshot,
    LiveMigration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridFeatureRow {
    pub request_kind: HybridRequestKind,
    pub backend: BackendPreference,
    pub supported_features: Vec<HybridFeatureKind>,
    pub missing_features: Vec<HybridFeatureKind>,
    pub feature_score: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridFeatureAudit {
    pub rows: Vec<HybridFeatureRow>,
    pub overall_feature_score: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridGapSeverity {
    Info,
    Warning,
    Critical,
}
