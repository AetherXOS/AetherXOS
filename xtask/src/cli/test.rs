use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum TestAction {
    QualityGate,
    /// Run host-side cargo check feature matrix validation
    Host {
        /// Run in release mode
        #[arg(long)]
        release: bool,
    },
    /// Run dashboard agent contract verification
    AgentContract,
    /// Run all core test tiers sequentially
    All {
        /// Use CI nextest profile and artifact-oriented behavior
        #[arg(long)]
        ci: bool,
    },
    /// POSIX conformance gate
    PosixConformance,
    /// Driver config smoke test
    DriverSmoke,
    /// Run a named CI tier locally or in GitHub Actions
    Tier {
        /// Tier name: fast, integration, nightly
        tier: String,
        /// Use CI nextest profile and artifact-oriented behavior
        #[arg(long)]
        ci: bool,
    },
    /// Linux app compatibility layered validator (strict/quick/qemu)
    LinuxAppCompat {
        #[arg(long)]
        desktop_smoke: bool,
        #[arg(long)]
        quick: bool,
        #[arg(long)]
        qemu: bool,
        #[arg(long)]
        strict: bool,
        #[arg(long)]
        ci: bool,
        #[arg(long)]
        require_busybox: bool,
        #[arg(long)]
        require_glibc: bool,
        #[arg(long)]
        require_wayland: bool,
        #[arg(long)]
        require_x11: bool,
        #[arg(long)]
        require_fs_stack: bool,
        #[arg(long)]
        require_package_stack: bool,
        #[arg(long)]
        require_desktop_app_stack: bool,
    },
    /// Audit kernel source for refactoring candidate areas
    KernelRefactorAudit {
        #[arg(long, default_value_t = crate::constants::defaults::audit::MAX_LINES)]
        max_lines: usize,
        #[arg(long, default_value_t = crate::constants::defaults::audit::MAGIC_REPEAT_THRESHOLD)]
        magic_repeat_threshold: usize,
    },
}
