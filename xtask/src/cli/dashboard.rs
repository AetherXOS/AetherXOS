use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DashboardAction {
    Live,
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
        match self {
            DashboardAction::Live => crate::utils::ui::dashboard::launch(),
            _ => crate::commands::dashboard::execute(self),
        }
    }
}
