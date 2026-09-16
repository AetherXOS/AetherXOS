use crate::utils::executable::Executable;
use anyhow::Result;
use clap::Args;

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

    /// Skip size and file count calculations (recommended for massive trees).
    #[arg(long)]
    pub no_stats: bool,

    /// Dry-run: show what would be deleted without actually removing files.
    #[arg(long)]
    pub dry_run: bool,

    /// Maximum directory depth to search for Cargo.toml files when purging build targets.
    #[arg(long, default_value_t = 3)]
    pub depth: usize,
}

impl Executable for CleanAction {
    fn execute(&self) -> Result<()> {
        crate::commands::ops::clean::execute(
            self.all,
            self.distros,
            self.logs,
            self.no_stats,
            self.dry_run,
            self.depth,
        )
    }
}
