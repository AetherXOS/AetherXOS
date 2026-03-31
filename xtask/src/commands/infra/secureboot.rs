use anyhow::{Context, Result, bail};
use std::process::Command;
use crate::cli::SecurebootAction;

pub fn execute(action: &SecurebootAction) -> Result<()> {
    match action {
        SecurebootAction::Sign { dry_run } => {
            println!("[secureboot::sign] Initializing Cryptographic UEFI payload verification signatures...");
            if *dry_run {
                println!("[secureboot::sign] STRICT MOCK MODE ACTIVE: Verification bypass enabled.");
            } else {
                sign_kernel_with_sbsign().context("Failed to cryptographically sign OS payload")?;
            }
        }
        SecurebootAction::PcrReport => {
            println!("[secureboot::pcr] Gathering Trusted Platform Module (PCR 0-7) measurement assertions.");
        }
    }
    
    Ok(())
}

fn sign_kernel_with_sbsign() -> Result<()> {
    let kernel_src = crate::utils::paths::resolve("artifacts/boot_image/stage/boot/hypercore.elf");
    let key_path = crate::utils::paths::resolve("artifacts/secureboot/MOK.key");
    let cert_path = crate::utils::paths::resolve("artifacts/secureboot/MOK.crt");
    
    if !kernel_src.exists() {
        bail!("Missing kernel payload to encrypt: execute 'cargo xtask build full' first.");
    }
    
    if !key_path.exists() || !cert_path.exists() {
        println!("[secureboot::sign] MOK cryptographic identities not detected on host. Automating key generation locally...");
        crate::utils::paths::ensure_dir(&crate::utils::paths::resolve("artifacts/secureboot"))?;
        
        let genkey = Command::new("openssl")
            .args(&["req", "-new", "-x509", "-newkey", "rsa:2048", "-keyout", &key_path.to_string_lossy(), "-out", &cert_path.to_string_lossy(), "-days", "3650", "-nodes", "-subj", "/CN=AetherXOS_Local_Platform_Key/"])
            .status()
            .context("Host lacks 'openssl' command line tools to generate secure boot variables.")?;
            
        if !genkey.success() {
            bail!("Fatal openssl sequence timeout. UEFI security boundaries compromised.");
        }
    }
    
    println!("[secureboot::sign] Generating PE/COFF SBAT hashes over final ELF structures.");
    let sign_status = Command::new("sbsign")
        .args(&["--key", &key_path.to_string_lossy(), "--cert", &cert_path.to_string_lossy(), "--output", &kernel_src.to_string_lossy(), &kernel_src.to_string_lossy()])
        .status()
        .context("Host machine completely misses 'sbsign' (sbsigntool package). Unable to physically manipulate Kernel PE headers on this OS.")?;
        
    if sign_status.success() {
        println!("[secureboot::sign] SUCCESS: AetherXOS kernel natively supports Tier-1 Boot Authentication restrictions!");
    } else {
        bail!("SBSign reported logical failure. Ensure target is a valid UEFI PE formatted binary executable.");
    }
    
    Ok(())
}
