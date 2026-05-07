use super::HybridRequestFamily;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridReleaseGateFamilyRow {
    pub family: HybridRequestFamily,
    pub min_coverage: u8,
    pub min_performance: u8,
    pub min_security: u8,
    pub high_risk_budget: usize,
    pub actual_coverage: u8,
    pub actual_performance: u8,
    pub actual_security: u8,
    pub high_risk_paths: usize,
    pub status: HybridReleaseGateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridReleaseGateMatrix {
    pub version: &'static str,
    pub rows: Vec<HybridReleaseGateFamilyRow>,
    pub system_rows: Vec<HybridReleaseGateSystemRow>,
    pub release_blocked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HybridReleaseGateSystemRow {
    pub name: &'static str,
    pub min_score: u8,
    pub actual_score: u8,
    pub blocker_count: usize,
    pub release_ready: bool,
    pub status: HybridReleaseGateStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridReleaseGateStatus {
    Pass,
    Warning,
    Block,
}
