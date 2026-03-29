mod cargo_metadata;
mod cli;
mod models;
mod userspace;

use std::fs;

fn main() -> Result<(), String> {
    let (repo_root, out_path, emit_dir, run_smoke) = cli::parse_args()?;
    let config = cargo_metadata::load_config_snapshot(&repo_root)?;
    let snapshot = models::CodegenSnapshot {
        config,
        userspace: userspace::userspace_snapshot(),
    };
    let rendered = serde_json::to_string_pretty(&snapshot)
        .map_err(|err| format!("failed to serialize snapshot: {err}"))?;
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(&out_path, rendered)
        .map_err(|err| format!("failed to write {}: {err}", out_path.display()))?;
    if let Some(dir) = emit_dir {
        userspace::emit_userspace_dir(&snapshot.userspace, &dir)?;
        if run_smoke {
            userspace::run_generated_userspace_smoke(&repo_root, &dir)?;
        }
    }
    Ok(())
}
