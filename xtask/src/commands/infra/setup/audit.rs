use crate::constants;
use crate::utils::process;
use anyhow::{Result, bail};

/// Evaluates the existing host workstation for critical OSDev tooling (QEMU, xorriso, cross-compilers).
pub(crate) fn audit_host_environment() -> Result<()> {
    println!("[setup::audit] Scanning host workstation architecture and active PATHs...");

    let required_bins = [
        constants::tools::QEMU_X86_64,
        constants::tools::XORRISO,
        constants::tools::RUSTC,
        constants::tools::CARGO,
    ];
    let mut missing = 0;

    for bin in required_bins {
        if process::which(bin) {
            println!(
                "[setup::audit] [ OK ] Verified binary executable inline: {}",
                bin
            );
        } else {
            println!(
                "[setup::audit] [FAIL] Critical system dependency missing: {}",
                bin
            );
            missing += 1;
        }
    }

    if missing > 0 {
        bail!(
            "Environment audit concluded with {} missing severe dependencies. Run 'cargo run -p xtask -- setup bootstrap' to inherently resolve these.",
            missing
        );
    }

    println!("[setup::audit] Success. Workstation meets all structural limits for OS engineering.");
    Ok(())
}
