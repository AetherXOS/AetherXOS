use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::utils::fs::hash::HashAlgo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroRegistry {
    pub version: String,
    pub description: String,
    pub distros: HashMap<String, Distro>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Distro {
    pub name: String,
    pub description: String,
    pub website: String,
    pub versions: HashMap<String, DistroVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistroVersion {
    #[serde(default)]
    pub codename: Option<String>,
    #[serde(rename = "type", default)]
    pub version_type: Option<String>,
    #[serde(default)]
    pub released: Option<String>,
    #[serde(default)]
    pub support_until: Option<String>,
    pub variants: HashMap<String, HashMap<String, Vec<DistroImage>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DistroImage {
    Simple(String),
    Detailed {
        url: String,
        #[serde(default)]
        hashes: HashMap<String, String>,
        #[serde(default)]
        sha256: Option<String>,
        #[serde(default)]
        size_bytes: Option<u64>,
    },
}

impl DistroImage {
    pub fn url(&self) -> &str {
        match self {
            Self::Simple(url) => url,
            Self::Detailed { url, .. } => url,
        }
    }

    pub fn size_bytes(&self) -> Option<u64> {
        match self {
            Self::Simple(_) => None,
            Self::Detailed { size_bytes, .. } => *size_bytes,
        }
    }

    pub fn hashes(&self) -> HashMap<HashAlgo, String> {
        let mut results = HashMap::new();
        if let Self::Detailed { hashes, sha256, .. } = self {
            for (algo_name, hash_str) in hashes {
                if let Some(algo) = HashAlgo::from_str(algo_name) {
                    results.insert(algo, hash_str.clone());
                }
            }
            if let Some(s) = sha256 {
                results.insert(HashAlgo::Sha256, s.clone());
            }
        }
        results
    }
}

impl DistroRegistry {
    pub fn load_default() -> Result<Self> {
        Self::load(Path::new("xtask/distro-registry.json"))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read registry: {}", path.display()))?;
        serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse registry: {}", path.display()))
    }

    pub fn collect_all_images(&self) -> Vec<(String, String, String, String, DistroImage)> {
        let mut results = Vec::new();
        for (dname, distro) in &self.distros {
            for (vname, version) in &distro.versions {
                for (varname, variant) in &version.variants {
                    for (arch, images) in variant {
                        for img in images {
                            results.push((dname.clone(), vname.clone(), varname.clone(), arch.clone(), img.clone()));
                        }
                    }
                }
            }
        }
        results
    }
}
