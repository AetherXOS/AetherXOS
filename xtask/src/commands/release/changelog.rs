use anyhow::{Result, Context};
use std::process::Command;
use crate::utils::logging;

pub fn generate_changelog(since_tag: Option<&str>) -> Result<String> {
    logging::status("RELEASE", "Extracting git commit history for changelog...");
    
    let mut args = vec!["log", "--pretty=format:* %s (%h)", "--no-merges"];
    if let Some(tag) = since_tag {
        args.push(&format!("{}..HEAD", tag));
    }

    let output = Command::new("git")
        .args(&args)
        .output()
        .context("Failed to run git log. Is git installed and is this a repo?")?;
        
    let history = String::from_utf8_lossy(&output.stdout);
    
    let mut changelog = format!("# Release Notes - {}\n\n", chrono::Local::now().format("%Y-%m-%d"));
    changelog.push_str("## Changes in this version:\n");
    changelog.push_str(&history);
    
    Ok(changelog)
}

pub fn save_changelog(content: &str) -> Result<()> {
    let path = crate::utils::paths::repo_root().join("CHANGELOG_DRAFT.md");
    std::fs::write(&path, content)?;
    logging::success("RELEASE", &format!("Changelog draft saved to {}", path.display()), &[]);
    Ok(())
}
