use crate::cli::Cli;
use crate::utils::executable::Executable;
use anyhow::Result;
use clap::{CommandFactory, Subcommand};
use clap_complete::{Shell, generate};
use std::io;

#[derive(Subcommand, Debug)]
pub enum CompletionAction {
    /// Generate autocomplete scripts for a specific shell.
    Generate {
        /// The shell to generate completions for.
        shell: Shell,
    },
}

impl Executable for CompletionAction {
    fn execute(&self) -> Result<()> {
        match self {
            CompletionAction::Generate { shell } => {
                let mut cmd = Cli::command();
                let name = cmd.get_name().to_string();
                generate(*shell, &mut cmd, name, &mut io::stdout());
                Ok(())
            }
        }
    }
}
