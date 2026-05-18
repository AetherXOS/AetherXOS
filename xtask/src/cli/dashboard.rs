use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum DashboardAction {
    Live,
    Build,
    Test,
    Open,
    Join {
        address: String,
    },
    AgentStart {
        #[arg(long)]
        no_safe: bool,
    },
}

impl crate::utils::executable::Executable for DashboardAction {
    fn execute(&self) -> anyhow::Result<()> {
        match self {
            DashboardAction::Live => crate::utils::ui::dashboard::launch(),
            DashboardAction::Join { address } => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(crate::utils::ui::collaborative::run_collaboration_client(
                    address,
                ))
            }
            _ => crate::commands::dashboard::execute(self),
        }
    }
}
