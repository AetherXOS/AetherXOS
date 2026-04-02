use crate::utils::logging;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Generate limine.conf and limine-probe.conf for the bootloader.
///
/// Replaces: scripts/limine_layout.py
pub fn generate_configs(
    stage_dir: &Path,
    kernel_name: &str,
    initramfs_name: &str,
    append_args: &str,
) -> Result<()> {
    let conf_path = stage_dir.join("limine.conf");
    let probe_path = stage_dir.join("limine-probe.conf");

    logging::info(
        "limine",
        "Generating configuration",
        &[("path", &conf_path.to_string_lossy())],
    );
    let conf = render_config(kernel_name, initramfs_name, append_args);
    fs::write(&conf_path, &conf)
        .with_context(|| format!("Failed to write {}", conf_path.display()))?;

    logging::info(
        "limine",
        "Generating configuration",
        &[("path", &probe_path.to_string_lossy())],
    );
    let probe_append = append_probe_args(append_args);
    let probe_conf = render_config(kernel_name, initramfs_name, &probe_append);
    fs::write(&probe_path, &probe_conf)
        .with_context(|| format!("Failed to write {}", probe_path.display()))?;

    logging::ready(
        "limine",
        "Configuration generation completed.",
        &stage_dir.to_string_lossy(),
    );
    Ok(())
}

/// Render the limine.conf content string.
fn render_config(kernel_name: &str, initramfs_name: &str, append: &str) -> String {
    format!(
        "default_entry: 1\n\
         timeout: 0\n\
         verbose: yes\n\
         serial: yes\n\
         serial_baudrate: 115200\n\
         graphics: no\n\
         \n\
         /AetherXOS\n\
         \x20   protocol: limine\n\
         \x20   kernel_path: boot():/boot/{kernel_name}\n\
         \x20   module_path: boot():/boot/{initramfs_name}\n\
         \x20   kernel_cmdline: {append}\n"
    )
}

/// Append the probe-mode flag to the kernel command line.
fn append_probe_args(append: &str) -> String {
    let flag = "HYPERCORE_RUN_LINKED_PROBE=1";
    if append.contains(flag) {
        append.to_string()
    } else {
        format!("{} {}", append, flag).trim().to_string()
    }
}
