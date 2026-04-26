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
}
