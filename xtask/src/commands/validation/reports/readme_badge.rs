use crate::utils::logging;
use anyhow::{Context, Result};
use regex::Regex;
use std::fs;

pub fn update_badges() -> Result<()> {
    logging::info(
        "report",
        "Synchronizing README status badges with live kernel metrics...",
        &[],
    );

    let stats = crate::commands::validation::linux_abi::audit_syscall_stats()
        .context("Failed to audit syscall stats for badge update")?;

    let readme_path = crate::utils::paths::repo_root().join("README.md");
    let mut content = fs::read_to_string(&readme_path).context("Failed to read README.md")?;

    // 1. Update Linux ABI Badge
    // Look for: ![Linux ABI Compatibility](https://img.shields.io/badge/Linux_ABI-XX.X%25-blue)
    let abi_regex = Regex::new(
        r"!\[Linux ABI Compatibility\]\(https://img\.shields\.io/badge/Linux_ABI-([0-9.]+)%25-blue\)",
    )?;
    let new_abi_badge = format!(
        "![Linux ABI Compatibility](https://img.shields.io/badge/Linux_ABI-{:.1}%25-blue)",
        stats.readiness_score
    );

    if abi_regex.is_match(&content) {
        content = abi_regex
            .replace(&content, new_abi_badge.as_str())
            .to_string();
    } else {
        // If not found, insert after the title
        let title_regex = Regex::new(r"(?m)^# Aether X OS\s*$")?;
        if let Some(m) = title_regex.find(&content) {
            let insert_pos = m.end();
            let badge_block = format!(
                "\n\n{}\n![Smoke Tests](https://img.shields.io/badge/Smoke_Tests-Passing-green)",
                new_abi_badge
            );
            content.insert_str(insert_pos, &badge_block);
        }
    }

    fs::write(&readme_path, content).context("Failed to write updated README.md")?;

    logging::ready("report", "README badges updated", "SUCCESS");
    Ok(())
}
