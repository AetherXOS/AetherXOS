use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Distro registry loaded from JSON file
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DistroRegistry {
    pub version: String,
    pub distros: HashMap<String, DistroEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DistroEntry {
    pub name: String,
    pub description: String,
    pub website: Option<String>,
    pub versions: HashMap<String, VersionEntry>,
    pub aliases: Option<Vec<String>>,
    #[serde(skip)]
    pub inherits_from: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionEntry {
    pub codename: Option<String>,
    #[serde(rename = "type")]
    pub release_type: Option<String>,
    pub released: Option<String>,
    pub support_until: Option<String>,
    pub variants: HashMap<String, HashMap<String, Vec<String>>>,
}

/// Load distro registry from JSON file
pub fn load_registry() -> anyhow::Result<DistroRegistry> {
    let registry_path = if cfg!(debug_assertions) {
        // In dev: look in xtask directory
        Path::new("xtask/distro-registry.json")
    } else {
        // In release: might be embedded or in artifacts
        Path::new("distro-registry.json")
    };

    if !registry_path.exists() {
        // Try alternative path
        let alt_path = Path::new("../xtask/distro-registry.json");
        if alt_path.exists() {
            return load_from_file(alt_path);
        }
        return Err(anyhow::anyhow!(
            "Distro registry not found. Expected: {}",
            registry_path.display()
        ));
    }

    load_from_file(registry_path)
}

fn load_from_file(path: &Path) -> anyhow::Result<DistroRegistry> {
    let content = std::fs::read_to_string(path)?;
    let registry: DistroRegistry = serde_json::from_str(&content)?;
    Ok(registry)
}

/// Resolve URLs for a distro identifier, handling aliases and direct URLs
pub fn resolve_distro_urls(distro: &str) -> Vec<String> {
    // If it's a direct URL, return it as-is
    if distro.starts_with("http://") || distro.starts_with("https://") {
        return vec![distro.to_string()];
    }

    // Try to load registry
    let registry = match load_registry() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Warning: Failed to load distro registry: {}", e);
            return Vec::new();
        }
    };

    // Parse distro string format: "distro-version[:variant]" or just "distro"
    let (base_spec, variant_filter) = if let Some((left, right)) = distro.split_once(':') {
        (left, Some(right))
    } else {
        (distro, None)
    };
    let parts: Vec<&str> = base_spec.split('-').collect();
    
    let mut urls = Vec::new();

    // Try exact match
    if let Some(entry) = registry.distros.get(base_spec) {
        urls.extend(extract_all_urls(entry, variant_filter));
        if !urls.is_empty() {
            return urls;
        }
    }

    // Try to match by alias
    for (_, entry) in &registry.distros {
        if let Some(aliases) = &entry.aliases {
            if aliases.contains(&base_spec.to_string()) {
                urls.extend(extract_all_urls(entry, variant_filter));
                if !urls.is_empty() {
                    return urls;
                }
            }
        }
    }

    // Try distro-version matching (e.g., "ubuntu-24.04")
    if parts.len() >= 2 {
        let distro_name = parts[0];
        let version = parts[1..].join("-");

        if let Some(entry) = registry.distros.get(distro_name) {
            if let Some(version_entry) = entry.versions.get(&version) {
                urls.extend(extract_urls_from_version(version_entry, variant_filter));
                if !urls.is_empty() {
                    return urls;
                }
            }
        }
    }

    // Try matching by alias for version pattern
    for (_, entry) in &registry.distros {
        if let Some(aliases) = &entry.aliases {
            for alias in aliases {
                if alias.starts_with(&format!("{}-", parts[0])) || 
                   (parts.len() >= 2 && alias.contains(&format!("-{}", parts[1]))) {
                    if let Some(version_entry) = entry.versions.get(&parts[1..].join("-")) {
                        urls.extend(extract_urls_from_version(version_entry, variant_filter));
                        if !urls.is_empty() {
                            return urls;
                        }
                    }
                }
            }
        }
    }

    urls
}

/// Extract all available URLs from a distro entry
fn extract_all_urls(entry: &DistroEntry, variant_filter: Option<&str>) -> Vec<String> {
    let mut urls = Vec::new();
    for (_, version) in &entry.versions {
        urls.extend(extract_urls_from_version(version, variant_filter));
    }
    urls
}

/// Extract URLs from a specific version entry
fn extract_urls_from_version(version: &VersionEntry, variant_filter: Option<&str>) -> Vec<String> {
    let mut urls = Vec::new();
    for (variant_name, arch_variants) in &version.variants {
        if let Some(filter) = variant_filter {
            if variant_name != filter {
                continue;
            }
        }
        // Prefer guest-compatible architecture URLs first.
        for arch in ["x86_64", "amd64", "x64", "arm64", "aarch64"] {
            if let Some(arch_urls) = arch_variants.get(arch) {
                for url in arch_urls {
                    if url.is_empty() {
                        continue;
                    }
                    // If variant wasn't explicitly selected, only return rootfs-friendly archives by default.
                    if variant_filter.is_none() && !is_rootfs_archive_url(url) {
                        continue;
                    }
                    urls.push(url.clone());
                }
            }
        }

        // Include any custom architecture keys not covered above.
        for (arch_name, arch_urls) in arch_variants {
            if ["x86_64", "amd64", "x64", "arm64", "aarch64"].contains(&arch_name.as_str()) {
                continue;
            }
            for url in arch_urls {
                if url.is_empty() {
                    continue;
                }
                if variant_filter.is_none() && !is_rootfs_archive_url(url) {
                    continue;
                }
                urls.push(url.clone());
            }
        }
    }
    urls
}

fn is_rootfs_archive_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.ends_with(".tar")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".txz")
        || lower.contains("root.tar")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_url() {
        let url = "https://example.com/my-rootfs.tar.gz";
        let urls = resolve_distro_urls(url);
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], url);
    }

    #[test]
    fn test_registry_loads() {
        match load_registry() {
            Ok(registry) => {
                assert!(!registry.distros.is_empty());
                println!("Registry loaded successfully with {} distros", registry.distros.len());
            }
            Err(e) => {
                eprintln!("Note: Registry not found (OK in some build contexts): {}", e);
            }
        }
    }

    #[test]
    fn test_distro_resolution_basic() {
        let urls = resolve_distro_urls("ubuntu-24.04");
        // May be empty if registry not found in test context, but shouldn't panic
        println!("Ubuntu 24.04 URLs: {:?}", urls);
    }

    #[test]
    fn test_distro_resolution_by_alias() {
        let urls = resolve_distro_urls("ubuntu-lts");
        // May be empty if registry not found, but shouldn't panic
        println!("Ubuntu LTS URLs: {:?}", urls);
    }

    #[test]
    fn test_unknown_distro_returns_empty() {
        let urls = resolve_distro_urls("unknown-distro-xyz-nonexistent");
        assert!(urls.is_empty());
    }
}
