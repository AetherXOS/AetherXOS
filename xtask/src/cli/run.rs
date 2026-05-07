use crate::types::Bootloader;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum RunAction {
    /// Execute robust QEMU pipeline targeting automated timeout evaluation loops
    Smoke {
        #[arg(long, default_value_t = Bootloader::Limine)]
        bootloader: Bootloader,
    },
    /// Provide graphical interactive emulator access allowing user UI validations
    Live {
        #[arg(long, default_value_t = crate::constants::defaults::run::FIRMWARE.to_string())]
        firmware: String,
    },
    /// Immediately stream compiled artifacts via block operations natively to an assigned storage drive
    BareMetalDeploy {
        #[arg(long)]
        device: String,
    },
    /// Launches QEMU in suspended execution mode and spawns a connected GDB instance automatically
    Debug {
        #[arg(long, default_value_t = crate::constants::defaults::run::FIRMWARE.to_string())]
        firmware: String,
    },
    /// Launches an ephemeral local network server facilitating PXE network booting for physical testing
    PxeServer {
        #[arg(long, default_value_t = crate::constants::defaults::run::PXE_PORT)]
        port: u16,
    },
    /// 🐧 Boot your kernel with a full Linux distribution (Ubuntu, Debian, Fedora, Alpine, etc.)
    /// 
    /// Supports downloading distros from a registry, caching them locally, or using a custom rootfs tarball.
    /// Automatically creates a partitioned disk image and attaches it to QEMU.
    /// 
    /// Examples:
    ///   cargo xtask run guest --distro ubuntu-24.04 --download
    ///   cargo xtask run guest --distro fedora-40 --download --refresh
    ///   cargo xtask run guest --rootfs ~/my-rootfs.tar.gz
    ///   cargo xtask run guest --distro debian-12 --download --firmware uefi
    /// 
    /// See `GUEST_GUIDE.md` for detailed documentation and troubleshooting.
    Guest {
        /// Distribution identifier from built-in registry (e.g., "ubuntu-24.04", "ubuntu-lts", "debian", "fedora-40", "alpine", etc.)
        /// Can also be a direct HTTPS URL to a rootfs tarball.
        /// Use --help to see all supported distros or check GUEST_GUIDE.md.
        #[arg(long, help = "Distro key or direct URL")]
        distro: Option<String>,

        /// Path to a local rootfs (directory or tarball). Takes priority over --distro.
        /// If this file doesn't exist, xtask will error clearly.
        #[arg(long, help = "Local rootfs path (dir or .tar.gz)")]
        rootfs: Option<String>,

        /// Download rootfs if not already cached. Requires curl or wget on the host.
        /// Falls back gracefully if download tools are unavailable.
        #[arg(long, default_value_t = false, help = "Download distro from registry")]
        download: bool,

        /// Cache downloaded rootfs locally at `artifacts/guest_cache/`.
        /// Disabling allows repeated downloads (not recommended).
        #[arg(long, default_value_t = true, help = "Use local cache (default: true)")]
        cache: bool,

        /// Force re-download even if distro is already cached.
        /// Useful to update to the latest version of a distro.
        #[arg(long, default_value_t = false, help = "Re-download even if cached")]
        refresh: bool,

        /// Explicitly attach the rootfs disk image as a virtio drive.
        /// Usually automatic; use this to force attachment if needed.
        #[arg(long, default_value_t = false, help = "Attach rootfs disk to QEMU")]
        attach: bool,

        /// Firmware mode: "bios" for legacy BIOS, "uefi" for UEFI (default: uefi).
        #[arg(long, default_value_t = crate::constants::defaults::run::FIRMWARE.to_string(), help = "Firmware: bios or uefi")]
        firmware: String,
    },
}

impl crate::utils::executable::Executable for RunAction {
    fn execute(&self) -> anyhow::Result<()> {
        crate::commands::ops::run::execute(self)
    }
}
