use crate::builders::secureboot::{openssl_mok_args, sbsign_args};
use crate::cli::SecurebootAction;
use crate::commands::validation;
use crate::constants;
use crate::utils::process;
use anyhow::{Context, Result, bail};

pub fn execute(action: &SecurebootAction) -> Result<()> {
    match action {
        SecurebootAction::Sign {
            dry_run,
            strict_verify,
        } => {
            println!(
                "[secureboot::sign] Initializing Cryptographic UEFI payload verification signatures..."
            );
            if *dry_run {
                println!(
                    "[secureboot::sign] STRICT MOCK MODE ACTIVE: Verification bypass enabled. (strict_verify={})",
                    strict_verify
                );
                Ok(())
            } else {
                execute_sign().context("Failed to cryptographically sign OS payload")
            }
        }
        SecurebootAction::PcrReport => {
            println!(
                "[secureboot::pcr] Gathering Trusted Platform Module (PCR 0-7) measurement assertions."
            );
            validation::secureboot::execute(action)
        }
        SecurebootAction::SbatValidate { .. }
        | SecurebootAction::MokPlan
        | SecurebootAction::OvmfMatrix { .. } => validation::secureboot::execute(action),
    }
}

fn execute_sign() -> Result<()> {
    let kernel_src = constants::paths::boot_image_stage_kernel();
    let secureboot_root = constants::paths::secureboot_root();
    let key_path = secureboot_root.join("MOK.key");
    let cert_path = secureboot_root.join("MOK.crt");

    if !kernel_src.exists() {
        bail!(
            "Missing kernel payload to encrypt: execute 'cargo run -p xtask -- build full' first."
        );
    }

    if !key_path.exists() || !cert_path.exists() {
        println!(
            "[secureboot::sign] MOK cryptographic identities not detected on host. Automating key generation locally..."
        );
        crate::utils::paths::ensure_dir(&secureboot_root)?;

        let key_args = openssl_mok_args(&key_path.to_string_lossy(), &cert_path.to_string_lossy());
        if !process::run_best_effort(
            "openssl",
            &key_args
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>(),
        ) {
            bail!("Fatal openssl sequence timeout. UEFI security boundaries compromised.");
        }
    }

    println!("[secureboot::sign] Generating PE/COFF SBAT hashes over final ELF structures.");
    let sign_args = sbsign_args(
        &key_path.to_string_lossy(),
        &cert_path.to_string_lossy(),
        &kernel_src.to_string_lossy(),
    );
    if process::run_best_effort(
        "sbsign",
        &sign_args
            .iter()
            .map(|value| value.as_str())
            .collect::<Vec<_>>(),
    ) {
        println!(
            "[secureboot::sign] SUCCESS: AetherXOS kernel natively supports Tier-1 Boot Authentication restrictions!"
        );
    } else {
        bail!(
            "SBSign reported logical failure. Ensure target is a valid UEFI PE formatted binary executable."
        );
    }

    Ok(())
}
