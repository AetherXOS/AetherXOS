use crate::cli::SecurebootAction;
use crate::commands::validation;
use crate::utils::logging;
use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn execute(action: &SecurebootAction) -> Result<()> {
    match action {
        SecurebootAction::Sign {
            dry_run,
            strict_verify,
        } => {
            logging::info("secureboot::sign", "Initializing Cryptographic UEFI payload verification signatures...", &[]);
            if *dry_run {
                logging::warn("secureboot::sign", &format!("STRICT MOCK MODE ACTIVE: Verification bypass enabled. (strict_verify={})", strict_verify), &[]);
                Ok(())
            } else {
                execute_sign().context("Failed to cryptographically sign OS payload")
            }
        }
        SecurebootAction::PcrReport => {
            logging::info("secureboot::pcr", "Gathering Trusted Platform Module (PCR 0-7) measurement assertions.", &[]);
            validation::secureboot::execute(action)
        }
        SecurebootAction::SbatValidate { .. }
        | SecurebootAction::MokPlan
        | SecurebootAction::OvmfMatrix { .. } => validation::secureboot::execute(action),
    }
}

fn execute_sign() -> Result<()> {
    let kernel_src = crate::utils::paths::resolve("artifacts/boot_image/stage/boot/hypercore.elf");
    let key_path = crate::utils::paths::resolve("artifacts/secureboot/MOK.key");
    let cert_path = crate::utils::paths::resolve("artifacts/secureboot/MOK.crt");

    if !kernel_src.exists() {
        bail!("Missing kernel payload to encrypt: execute 'cargo run -p xtask -- build full' first.");
    }

    if !key_path.exists() || !cert_path.exists() {
        logging::warn("secureboot::sign", "MOK cryptographic identities not detected on host. Automating key generation locally...", &[]);
        crate::utils::paths::ensure_dir(&crate::utils::paths::resolve("artifacts/secureboot"))?;

        let genkey = Command::new("openssl")
            .args(&[
                "req",
                "-new",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-keyout",
                &key_path.to_string_lossy(),
                "-out",
                &cert_path.to_string_lossy(),
                "-days",
                "3650",
                "-nodes",
                "-subj",
                "/CN=AetherXOS_Local_Platform_Key/",
            ])
            .status()
            .context(
                "Host lacks 'openssl' command line tools to generate secure boot variables.",
            )?;

        if !genkey.success() {
            bail!("Fatal openssl sequence timeout. UEFI security boundaries compromised.");
        }
    }

    logging::info("secureboot::sign", "Generating PE/COFF SBAT hashes over final ELF structures.", &[]);
    let sign_status = Command::new("sbsign")
        .args(&["--key", &key_path.to_string_lossy(), "--cert", &cert_path.to_string_lossy(), "--output", &kernel_src.to_string_lossy(), &kernel_src.to_string_lossy()])
        .status()
        .context("Host machine completely misses 'sbsign' (sbsigntool package). Unable to physically manipulate Kernel PE headers on this OS.")?;

    if sign_status.success() {
        logging::ready("secureboot::sign", "AetherXOS kernel natively supports Tier-1 Boot Authentication restrictions", &kernel_src.to_string_lossy());
    } else {
        bail!("SBSign reported logical failure. Ensure target is a valid UEFI PE formatted binary executable.");
    }

    Ok(())
}
