use anyhow::Result;
use clap::Args;
use crate::utils::executable::Executable;

#[derive(Args, Debug, Clone)]
pub struct CleanAction {
    /// Perform a deep clean, including cargo build artifacts and caches.
    #[arg(long, short = 'a')]
    pub all: bool,

    /// Specifically target downloaded distro rootfs and ISO images.
    #[arg(long, short = 'd')]
    pub distros: bool,

    /// Remove all xtask execution logs.
    #[arg(long, short = 'l')]
    pub logs: bool,

    /// Dry-run: show what would be deleted without actually removing files.
    #[arg(long)]
    pub dry_run: bool,
}

impl Executable for CleanAction {
    fn execute(&self) -> Result<()> {
        crate::commands::ops::clean::execute(self.all, self.distros, self.logs, self.dry_run)
    }
}
