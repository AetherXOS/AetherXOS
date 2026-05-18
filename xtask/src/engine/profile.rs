use crate::engine::ExecutionContext;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildProfile {
    pub name: String,
    pub arch: String,
    pub features: Vec<String>,
    pub parameters: HashMap<String, String>,
}

impl BuildProfile {
    pub fn save(&self, root: &Path) -> Result<()> {
        let profiles_dir = root.join(".xtask").join("profiles");
        fs::create_dir_all(&profiles_dir)?;

        let path = profiles_dir.join(format!("{}.json", self.name));
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn load(root: &Path, name: &str) -> Result<Self> {
        let path = root
            .join(".xtask")
            .join("profiles")
            .join(format!("{}.json", name));
        let json = fs::read_to_string(path)?;
        let profile = serde_json::from_str(&json)?;
        Ok(profile)
    }

    pub fn list(root: &Path) -> Vec<String> {
        let profiles_dir = root.join(".xtask").join("profiles");
        if !profiles_dir.exists() {
            return Vec::new();
        }

        fs::read_dir(profiles_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                    .filter_map(|e| {
                        e.path()
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn apply_to(&self, ctx: &mut ExecutionContext) {
        ctx.arch = self.arch.clone();
        ctx.features = self.features.clone();
        ctx.parameters = self.parameters.clone();
    }
}
