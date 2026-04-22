use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum SetupAction {
    Audit,
    Repair,
    Bootstrap,
    InstallerSelect {
        /// Installer profile id (for example: debian, ubuntu-base, arch)
        #[arg(long, default_value = "debian")]
        profile: String,
        /// Optional comma-separated app targets (for example: python,chrome)
        #[arg(long)]
        apps: Option<String>,
        /// Optional comma-separated package override list (replaces profile defaults)
        #[arg(long)]
        packages: Option<String>,
        /// Optional comma-separated package include list
        #[arg(long)]
        include: Option<String>,
        /// Optional comma-separated package exclude list
        #[arg(long)]
        exclude: Option<String>,
        /// Optional mirror override
        #[arg(long)]
        mirror: Option<String>,
        /// Optional output path for generated selection JSON
        #[arg(long)]
        out: Option<String>,
    },
    /// Synchronize and provision external vendored bootloaders (e.g., pulling Limine from GitHub releases)
    FetchBootloader,
    /// Provision target compilers required for host cross-compilation boundaries
    Toolchain,
}
