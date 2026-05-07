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

impl crate::utils::executable::Executable for DashboardAction {
    fn execute(&self) -> anyhow::Result<()> {
        crate::commands::dashboard::execute(self)
    }
}
