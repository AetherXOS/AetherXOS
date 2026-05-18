use crate::utils::executable::Executable;
use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub enum InteractiveAction {
    /// Launch the interactive TUI (default)
    Launch,
}

impl Executable for InteractiveAction {
    fn execute(&self) -> anyhow::Result<()> {
        match self {
            InteractiveAction::Launch => crate::commands::interactive::menu::launch_main_menu(),
        }
    }
}
