use crate::types::{Bootloader, ImageFormat};
use aethercore_common::TargetArch;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum BuildAction {
    /// Integrates OS elements (Kernel + RootFS) into an immediately bootable payload target
    Full {
        /// Explicit Host/Guest compiler target architecture (e.g., x86_64, aarch64)
        #[arg(long, default_value_t = TargetArch::X86_64)]
        arch: TargetArch,

        /// Assigned boot target application protocol for OS handoff
        #[arg(long, default_value_t = Bootloader::Limine)]
        bootloader: Bootloader,

        /// Format boundary wrapper generated for virtualization or raw deployment
        #[arg(long, default_value_t = ImageFormat::Iso)]
        format: ImageFormat,

        /// Enable specific kernel feature gates (comma-separated: vfs,drivers,net,smp,test_mode)
        #[arg(long, default_value = "vfs,drivers,logging,syscalls")]
        features: aethercore_common::KernelFeatures,

        /// Toggle LLVM/Rust optimization profiles flag
        #[arg(long)]
        release: bool,
        /// Optional external guest rootfs (directory or tarball) to include in image
        #[arg(long)]
        rootfs: Option<String>,
    },

    /// Aggregates existing pre-built objects strictly for Image wrapper creation
    Image {
        #[arg(long, default_value_t = Bootloader::Limine)]
        bootloader: Bootloader,

        #[arg(long, default_value_t = ImageFormat::Iso)]
        format: ImageFormat,
    },

    /// Instructs the compiler to strictly compile the Kernel ELF void of external wrappers
    Kernel {
        #[arg(long, default_value_t = TargetArch::X86_64)]
        arch: TargetArch,

        /// Enable specific kernel feature gates
        #[arg(long, default_value = "vfs,drivers,logging,syscalls")]
        features: aethercore_common::KernelFeatures,

        #[arg(long)]
        release: bool,
    },

    /// Archives core userspace modules into the pre-mount Initial RAM filesystem layout
    Initramfs,

    /// Compiles a dedicated userspace program and bundles it directly into the initramfs layout
    App {
        /// The exact package/crate name of the userspace application to construct
        name: String,

        /// Toggle LLVM/Rust optimization profiles flag
        #[arg(long)]
        release: bool,
    },

    /// Create a distro-based ISO (e.g., Ubuntu) that uses the AetherXOS kernel
    DistroIso {
        /// Distro name (e.g., ubuntu, debian)
        #[arg(long)]
        distro: Option<String>,
        /// Distro version (e.g., 24.04, 22.04)
        #[arg(long)]
        version: Option<String>,
        /// Variant (e.g., minimal, server)
        #[arg(long)]
        variant: Option<String>,
        /// Architecture
        #[arg(long)]
        arch: Option<TargetArch>,
    },

    /// Replace the kernel inside an existing ISO image without rebuilding the whole ISO.
    /// Useful for fast kernel iteration: extracts ISO, swaps `boot/aethercore.elf`, and re-packages.
    UpdateIsoKernel {
        /// Path to existing ISO to update
        #[arg(long)]
        iso: String,

        /// Optional path to a pre-built kernel ELF (skips rebuild)
        #[arg(long)]
        kernel: Option<String>,

        /// Optional output ISO path (defaults to <iso>-updated.iso)
        #[arg(long)]
        out: Option<String>,
        /// Optional working directory to extract ISO into (use a drive with free space)
        #[arg(long)]
        workdir: Option<String>,
    },

    /// Generate P0/P1/P2 tier readiness status
    TierStatus,

    /// Rebuild the kernel ELF and run a full ELF integrity + security audit.
    /// Faster than a full ISO build — ideal for iterating on kernel changes.
    #[command(name = "verify-elf")]
    VerifyElf {
        /// Target architecture to build and verify (default: x86_64)
        #[arg(long, default_value_t = TargetArch::X86_64)]
        arch: TargetArch,

        /// Build in release mode before verification
        #[arg(long)]
        release: bool,

        /// Path to a pre-built ELF binary (skips rebuild, just verifies)
        #[arg(long)]
        elf: Option<String>,
    },
}

impl crate::utils::executable::Executable for BuildAction {
    fn execute(&self) -> anyhow::Result<()> {
        crate::commands::infra::build::execute(self)
    }
}
