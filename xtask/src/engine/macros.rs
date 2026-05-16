use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use crate::utils::logging;

pub struct Macro {
    pub name: String,
    pub commands: Vec<String>,
}

pub fn record_macro(name: &str, commands: Vec<String>) -> Result<()> {
    let path = get_macro_path(name);
    let content = commands.join("\n");
    fs::write(path, content)?;
    logging::success("MACRO", &format!("Macro '{}' recorded with {} commands", name, commands.len()), &[]);
    Ok(())
}

pub fn replay_macro(name: &str) -> Result<()> {
    let path = get_macro_path(name);
    if !path.exists() {
        anyhow::bail!("Macro '{}' not found", name);
    }
    
    let content = fs::read_to_string(path)?;
    let commands: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    
    logging::status("MACRO", &format!("Replaying macro: {}", name));
    for cmd in commands {
        logging::info("MACRO", &format!("Executing: {}", cmd), &[]);
        // In a real impl, we would parse and execute the CLI commands here.
        // For now, we'll just log it.
    }
    
    Ok(())
}

fn get_macro_path(name: &str) -> PathBuf {
    crate::utils::paths::repo_root().join(".xtask").join("macros").join(format!("{}.macro", name))
}
