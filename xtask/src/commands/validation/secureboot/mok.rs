use anyhow::Result;

use crate::constants;
use crate::utils::paths;
use crate::utils::report;

pub fn mok_plan() -> Result<()> {
    println!("[secureboot::mok] Generating MOK enrollment plan");

    let cert = paths::resolve("keys/MOK.cer");
    let out_dir = constants::paths::secureboot_root();
    paths::ensure_dir(&out_dir)?;

    let steps = [
        "1) Copy certificate to target machine.",
        "2) Run: mokutil --import <cert>",
        "3) Reboot and enroll key in MOK Manager UI.",
        "4) Verify: mokutil --list-enrolled",
        "5) Reboot and validate shim/grub/loader path.",
    ];

    // Write markdown plan
    let mut md = String::new();
    md.push_str("# Secure Boot MOK Enrollment Plan\n\n");
    md.push_str(&format!("- cert_path: `{}`\n\n", cert.display()));
    md.push_str("## Steps\n\n");
    for step in &steps {
        md.push_str(&format!("- {}\n", step));
    }
    md.push_str("\n## Commands\n\n");
    md.push_str(&format!("- `mokutil --import {}`\n", cert.display()));
    md.push_str("- `mokutil --list-enrolled`\n");
    md.push_str("- `mokutil --test-key <cert>`\n");

    report::write_text_report(&out_dir.join("mok_plan.md"), &md)?;
    println!(
        "[secureboot::mok] Plan written: {}",
        out_dir.join("mok_plan.md").display()
    );
    Ok(())
}
