use crate::utils::logging;
use anyhow::Result;
use std::env;

pub fn execute() -> Result<()> {
    logging::info("info", "collecting environment diagnostics...", &[]);

    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let cpu_model = crate::get_cpu_model();
    let rustc_version = crate::get_rustc_version();
    let version = env!("CARGO_PKG_VERSION");

    let qemu_system_x86_64_present = crate::utils::process::which("qemu-system-x86_64")
        || crate::utils::process::which("qemu-system-x86_64.exe");
    let qemu_img_present =
        crate::utils::process::which("qemu-img") || crate::utils::process::which("qemu-img.exe");
    let xorriso_present =
        crate::utils::process::which("xorriso") || crate::utils::process::which("xorriso.exe");

    let status = |present: bool| if present { "Installed" } else { "Missing" };

    println!();
    println!("--- Aether X OS Environment Info ---");
    println!("XTask Version:  {}", version);
    println!("OS:             {}", os);
    println!("Architecture:   {}", arch);
    println!("CPU Model:      {}", cpu_model);
    println!("Rustc Version:  {}", rustc_version);
    println!();
    println!("--- Dependencies ---");
    println!("qemu-system:    {}", status(qemu_system_x86_64_present));
    println!("qemu-img:       {}", status(qemu_img_present));
    println!("xorriso:        {}", status(xorriso_present));
    println!("------------------------------------");
    println!();

    Ok(())
}
