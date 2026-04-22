use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DashboardAction {
    Build,
    Test,
    Open,
    AgentStart {
        #[arg(long)]
        no_safe: bool,
    },
}
