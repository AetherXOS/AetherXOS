use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum LinuxAbiAction {
    GapInventory,
    Gate,
    ErrnoConformance,
    ShimErrnoConformance,
    ReadinessScore,
    P2GapReport,
    P2GapGate,
    SemanticMatrix,
    TrendDashboard {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        strict: bool,
    },
    WorkloadCatalog {
        #[arg(long, default_value_t = 30)]
        limit: usize,
        #[arg(long)]
        strict: bool,
    },
}
