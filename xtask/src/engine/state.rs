use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// Unused imports removed

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct EngineState {
    pub task_hashes: HashMap<String, String>,
    pub remote_cache_url: Option<String>,
}

impl EngineState {
    pub fn load() -> Self {
        let path = crate::utils::core::context::out_dir().join(".xtask_state.json");
        if !path.exists() {
            return Self {
                task_hashes: HashMap::new(),
                remote_cache_url: None,
            };
        }

        let raw = std::fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = crate::utils::core::context::out_dir().join(".xtask_state.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json).context("Failed to save engine state")
    }

    pub fn get_hash(&self, task_name: &str) -> Option<&String> {
        self.task_hashes.get(task_name)
    }

    pub fn get_all_hashes(&self) -> &HashMap<String, String> {
        &self.task_hashes
    }

    pub fn set_hash(&mut self, task_name: String, hash: String) {
        self.task_hashes.insert(task_name, hash);
    }
}
