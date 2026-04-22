use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct EvidenceFileEntry {
    pub path: String,
    pub required: bool,
    pub exists: bool,
    pub size_bytes: Option<u64>,
    pub sha256: Option<String>,
    pub modified_utc: Option<String>,
    pub gate_ok: Option<bool>,
    pub gate_detail: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ReproducibleBuildEvidence {
    pub generated_utc: String,
    pub git_commit: Option<String>,
    pub rustc_version: String,
    pub cargo_version: String,
    pub host_os: String,
    pub host_arch: String,
    pub files: Vec<EvidenceFileEntry>,
    pub missing_files: usize,
}

#[derive(Serialize)]
pub struct ReleaseEvidenceBundle {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub required_missing: usize,
    pub required_gate_failures: usize,
    pub missing_required: Vec<String>,
    pub failing_required_gates: Vec<String>,
    pub entries: Vec<EvidenceFileEntry>,
}

#[derive(Serialize)]
pub struct ReleaseDiagnosticIssue {
    pub id: String,
    pub severity: String,
    pub source: String,
    pub detail: String,
    pub remediation: String,
}

#[derive(Serialize)]
pub struct ReleaseDiagnosticsReport {
    pub generated_utc: String,
    pub overall_ok: bool,
    pub strict: bool,
    pub issue_count: usize,
    pub issues: Vec<ReleaseDiagnosticIssue>,
}

#[derive(Serialize)]
pub struct PolicyViolation {
    pub path: String,
    pub line: usize,
    pub pattern: String,
    pub severity: String,
    pub snippet: String,
}

#[derive(Serialize)]
pub struct PolicyGuardReport {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub violation_count: usize,
    pub scanned_files: usize,
    pub violations: Vec<PolicyViolation>,
}

#[derive(Serialize)]
pub struct WarningAuditHit {
    pub source_file: String,
    pub line: String,
}

#[derive(Serialize)]
pub struct WarningAuditReport {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub scanned_logs: usize,
    pub hit_count: usize,
    pub hits: Vec<WarningAuditHit>,
}

#[derive(Serialize)]
pub struct CiBundleDoc {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub checks: Vec<BundleCheck>,
}

#[derive(Serialize)]
pub struct BundleCheck {
    pub id: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Serialize)]
pub struct DoctorReport {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub checks: Vec<BundleCheck>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TrendPoint {
    pub generated_utc: String,
    pub overall_ok: bool,
    pub failed_count: usize,
    pub completion_pct: f64,
}

#[derive(Serialize)]
pub struct TrendDashboardDoc {
    pub generated_utc: String,
    pub strict: bool,
    pub points: Vec<TrendPoint>,
    pub latest_overall_ok: bool,
    pub latest_failed_count: usize,
    pub regression_detected: bool,
}

#[derive(Serialize)]
pub struct FreezeCheckDoc {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub branch: String,
    pub worktree_clean: bool,
    pub detail: String,
}

#[derive(Serialize)]
pub struct SbomAuditDoc {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub package_count: usize,
    pub duplicate_name_count: usize,
    pub top_package_names: Vec<String>,
}

#[derive(Serialize)]
pub struct ScoreNormalizeDoc {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub host_os: String,
    pub host_arch: String,
    pub raw_completion_pct: f64,
    pub normalized_score: f64,
    pub failed_checks: usize,
}

#[derive(Serialize)]
pub struct PerfEngineeringReportDoc {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub gate_completion_pct: f64,
    pub normalized_gate_score: f64,
    pub failed_checks: usize,
    pub release_regression_detected: bool,
    pub linux_abi_score: f64,
    pub perf_engineering_score: f64,
    pub threshold_min_perf_score: f64,
    pub threshold_min_normalized_gate_score: f64,
    pub threshold_max_failed_checks: usize,
    pub waiver_allow_regression: bool,
    pub waiver_allow_below_min_score: bool,
    pub threshold_source: String,
    pub waiver_source: String,
}

#[derive(Serialize, Deserialize)]
pub struct PerfThresholdConfig {
    pub min_perf_engineering_score: f64,
    pub min_normalized_gate_score: f64,
    pub max_failed_checks: usize,
}

impl Default for PerfThresholdConfig {
    fn default() -> Self {
        Self {
            min_perf_engineering_score: 90.0,
            min_normalized_gate_score: 94.0,
            max_failed_checks: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct PerfWaiverConfig {
    pub waiver_id: Option<String>,
    pub reason: Option<String>,
    pub allow_regression: bool,
    pub allow_below_min_score: bool,
}

#[derive(Serialize)]
pub struct ReleaseManifestDoc {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub git_commit: Option<String>,
    pub host_os: String,
    pub host_arch: String,
    pub required_missing: usize,
    pub required_files: Vec<EvidenceFileEntry>,
}
