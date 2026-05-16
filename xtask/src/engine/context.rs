use std::path::PathBuf;
use crate::utils::core::context as app_ctx;

pub struct ExecutionContext {
    pub repo_root: PathBuf,
    pub out_dir: PathBuf,
    pub is_release: bool,
    pub arch: String,
    pub features: Vec<String>,
    pub staging: Option<super::staging::StagingArea>,
    pub state: super::state::EngineState,
}

impl ExecutionContext {
    pub fn from_defaults() -> Self {
        Self {
            repo_root: app_ctx::repo_root(),
            out_dir: app_ctx::out_dir(),
            is_release: false,
            arch: "x86_64".to_string(),
            features: Vec::new(),
            staging: None,
            state: super::state::EngineState::load(),
        }
    }

    pub fn resolve_target_binary(&self, _package: &str, bin: &str) -> PathBuf {
        let profile = if self.is_release { "release" } else { "debug" };
        self.repo_root
            .join("target")
            .join(&self.arch)
            .join(profile)
            .join(bin)
    }

    pub fn artifact_path(&self, name: &str) -> PathBuf {
        self.out_dir.join(name)
    }
}
