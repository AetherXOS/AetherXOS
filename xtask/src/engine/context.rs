use std::path::PathBuf;
use std::sync::RwLock;
use crate::utils::fs::paths::LAYOUT;

#[derive(Clone)]
pub struct ExecutionContext {
    pub repo_root: PathBuf,
    pub out_dir: PathBuf,
    pub is_release: bool,
    pub arch: String,
    pub features: Vec<String>,
    pub staging: Option<super::staging::StagingArea>,
    pub state: std::sync::Arc<RwLock<super::state::EngineState>>,
    pub non_interactive: bool,
    pub dry_run: bool,
    pub parameters: std::collections::HashMap<String, String>,
}

impl ExecutionContext {
    pub fn from_defaults() -> Self {
        Self {
            repo_root: LAYOUT.root.clone(),
            out_dir: LAYOUT.artifacts.clone(),
            is_release: false,
            arch: "x86_64".to_string(),
            features: Vec::new(),
            staging: None,
            state: std::sync::Arc::new(RwLock::new(super::state::EngineState::load())),
            non_interactive: false,
            dry_run: false,
            parameters: std::collections::HashMap::new(),
        }
    }

    pub fn resolve_target_binary(&self, _package: &str, bin: &str) -> PathBuf {
        let profile = if self.is_release { "release" } else { "debug" };
        LAYOUT.target
            .join(&self.arch)
            .join(profile)
            .join(bin)
    }

    pub fn artifact_path(&self, name: &str) -> PathBuf {
        self.out_dir.join(name)
    }
}
