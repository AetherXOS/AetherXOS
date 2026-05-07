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
    /// Alias for FetchBootloader — fetch and verify Limine bootloader binaries
    #[command(name = "limine")]
    Limine,
    /// Provision target compilers required for host cross-compilation boundaries
    Toolchain,
    /// Perform a deep check of the distro registry (JSON schema, mandatory fields, etc.)
    DistroCheck {
        /// Optional path to registry JSON (defaults to xtask/distro-registry.json)
        #[arg(long)]
        file: Option<String>,
    },
    /// Validate distro registry URLs for availability and content length
    DistroUrlChecks {
        /// Optional path to registry JSON (defaults to xtask/distro-registry.json)
        #[arg(long)]
        file: Option<String>,
        /// Write JSON report to this path (default: artifacts/distro_url_report.json)
        #[arg(long)]
        out: Option<String>,
        /// Enable size verification against registry metadata
        #[arg(long)]
        verify_size: bool,
    },
}

impl crate::utils::executable::Executable for SetupAction {
    fn execute(&self) -> anyhow::Result<()> {
        crate::commands::infra::setup::execute(self)
    }
}
