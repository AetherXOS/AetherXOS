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

        /// Toggle LLVM/Rust optimization profiles flag
        #[arg(long)]
        release: bool,
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

    /// Generate P0/P1/P2 tier readiness status
    TierStatus,
}
