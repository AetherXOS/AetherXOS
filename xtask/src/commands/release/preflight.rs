use anyhow::Result;

pub mod abi;
pub mod ci;
pub mod diagnostics;
pub mod docs_audit;
pub mod evidence_bundle;
pub mod gates;
pub mod host_tools;
pub mod models;
pub mod reports;
pub mod repro_compare;

pub(crate) use ci::{
    build_file_entry, ci_bundle, evaluate_gate, parse_csv_lower, relative_display,
    render_bundle_md, reproducible_evidence,
};
pub(crate) use diagnostics::{release_diagnostics, release_doctor};
pub(crate) use models::{ReleaseEvidenceBundle, ReproducibleBuildEvidence};
pub(crate) use reports::export_junit;

use crate::cli::ReleaseAction;

pub fn execute(action: &ReleaseAction) -> Result<()> {
    match action {
        ReleaseAction::Preflight {
            skip_host_tests,
            skip_boot_artifacts,
            strict_production_gate,
        } => gates::preflight(
            *skip_host_tests,
            *skip_boot_artifacts,
            *strict_production_gate,
        ),
        ReleaseAction::CandidateGate => gates::candidate_gate(),
        ReleaseAction::P0Gate => gates::p0_gate(),
        ReleaseAction::P0Acceptance => gates::p0_acceptance(),
        ReleaseAction::P1Nightly => gates::p1_nightly(),
        ReleaseAction::P1Acceptance => gates::p1_acceptance(),
        ReleaseAction::P0P1Nightly => gates::p0_p1_nightly(),
        ReleaseAction::ReproducibleEvidence => ci::reproducible_evidence(),
        ReleaseAction::ReproducibilityCompare {
            strict,
            host_matrix,
            inputs,
        } => repro_compare::run(*strict, host_matrix.as_deref(), inputs.as_deref()),
        ReleaseAction::DocsCommandAudit { strict } => docs_audit::run(*strict),
        ReleaseAction::EvidenceBundle { strict } => evidence_bundle::run(*strict),
        ReleaseAction::AbiDriftReport { baseline, strict } => {
            abi::abi_drift_report(baseline.as_deref(), *strict)
        }
        ReleaseAction::Diagnostics { strict } => diagnostics::release_diagnostics(*strict),
        ReleaseAction::HostToolVerify { strict } => host_tools::host_tool_verify(*strict),
        ReleaseAction::PolicyGuard { strict } => diagnostics::critical_policy_guard(*strict),
        ReleaseAction::WarningAudit { strict, from_file } => {
            diagnostics::warning_audit(*strict, from_file.as_deref())
        }
        ReleaseAction::GateFixup { strict } => ci::gate_fixup(*strict),
        ReleaseAction::CiBundle { strict } => ci::ci_bundle(*strict),
        ReleaseAction::Doctor { strict } => diagnostics::release_doctor(*strict),
        ReleaseAction::GateReport { prev, strict } => {
            reports::gate_report(prev.as_deref(), *strict)
        }
        ReleaseAction::ExportJunit { out, strict } => {
            reports::export_junit(out.as_deref(), *strict)
        }
        ReleaseAction::ExplainFailure { strict } => reports::explain_failure(*strict),
        ReleaseAction::TrendDashboard { limit, strict } => {
            reports::trend_dashboard(*limit, *strict)
        }
        ReleaseAction::PerfReport { strict } => reports::perf_report(*strict),
        ReleaseAction::FreezeCheck {
            strict,
            allow_dirty,
        } => reports::freeze_check(*strict, *allow_dirty),
        ReleaseAction::SbomAudit { strict } => reports::sbom_audit(*strict),
        ReleaseAction::ScoreNormalize { strict } => reports::score_normalize(*strict),
        ReleaseAction::ReleaseNotes { out } => reports::release_notes(out.as_deref()),
        ReleaseAction::ReleaseManifest { strict } => reports::release_manifest(*strict),
        ReleaseAction::SupportDiagnostics { strict } => reports::support_diagnostics(*strict),
        ReleaseAction::AbiPerfGate { strict } => gates::abi_perf_gate(*strict),
    }
}
