use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ReleaseAction {
    Preflight {
        #[arg(long)]
        skip_host_tests: bool,
        #[arg(long)]
        skip_boot_artifacts: bool,
        #[arg(long)]
        strict_production_gate: bool,
    },
    CandidateGate,
    P0Gate,
    P0Acceptance,
    P1Nightly,
    P1Acceptance,
    P0P1Nightly,
    ReproducibleEvidence,
    ReproducibilityCompare {
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        host_matrix: Option<String>,
        #[arg(long)]
        inputs: Option<String>,
    },
    DocsCommandAudit {
        #[arg(long)]
        strict: bool,
    },
    EvidenceBundle {
        #[arg(long)]
        strict: bool,
    },
    AbiDriftReport {
        #[arg(long)]
        baseline: Option<String>,
        #[arg(long)]
        strict: bool,
    },
    Diagnostics {
        #[arg(long)]
        strict: bool,
    },
    HostToolVerify {
        #[arg(long)]
        strict: bool,
    },
    PolicyGuard {
        #[arg(long)]
        strict: bool,
    },
    WarningAudit {
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        from_file: Option<String>,
    },
    GateFixup {
        #[arg(long)]
        strict: bool,
    },
    CiBundle {
        #[arg(long)]
        strict: bool,
    },
    Doctor {
        #[arg(long)]
        strict: bool,
    },
    GateReport {
        #[arg(long)]
        prev: Option<String>,
        #[arg(long)]
        strict: bool,
    },
    ExportJunit {
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        strict: bool,
    },
    ExplainFailure {
        #[arg(long)]
        strict: bool,
    },
    TrendDashboard {
        #[arg(long, default_value_t = 30)]
        limit: usize,
        #[arg(long)]
        strict: bool,
    },
    FreezeCheck {
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        allow_dirty: bool,
    },
    SbomAudit {
        #[arg(long)]
        strict: bool,
    },
    ScoreNormalize {
        #[arg(long)]
        strict: bool,
    },
    ReleaseNotes {
        #[arg(long)]
        out: Option<String>,
    },
    ReleaseManifest {
        #[arg(long)]
        strict: bool,
    },
    SupportDiagnostics {
        #[arg(long)]
        strict: bool,
    },
    AbiPerfGate {
        #[arg(long)]
        strict: bool,
    },
    PerfReport {
        #[arg(long)]
        strict: bool,
    },
}
