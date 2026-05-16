use anyhow::{Context, Result};
use aethercore_common::KernelFeatures;
use std::collections::BTreeSet;

use crate::utils::{logging, paths, ui};

#[derive(Clone, Debug)]
pub struct FeatureCatalog {
    pub all: Vec<String>,
    pub default: Vec<String>,
    pub kernel_all: Vec<String>,
    pub kernel_default: Vec<String>,
}

#[derive(Clone, Copy)]
pub enum KernelFeatureSelectionMode {
    Default,
    All,
    Custom,
}

#[derive(Clone, Copy)]
pub enum CargoFeatureSelectionMode {
    Default,
    All,
    Custom,
}

impl core::fmt::Display for CargoFeatureSelectionMode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Default => write!(f, "Default features (from Cargo.toml)"),
            Self::All => write!(f, "All features (from Cargo.toml)"),
            Self::Custom => write!(f, "Custom selection"),
        }
    }
}

impl core::fmt::Display for KernelFeatureSelectionMode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Default => write!(f, "Default features (from Cargo.toml)"),
            Self::All => write!(f, "All kernel features (from Cargo.toml)"),
            Self::Custom => write!(f, "Custom selection"),
        }
    }
}

pub fn load_feature_catalog() -> Result<FeatureCatalog> {
    let cargo_toml_path = paths::repo_root().join("Cargo.toml");
    let text = std::fs::read_to_string(&cargo_toml_path)
        .with_context(|| format!("failed to read {}", cargo_toml_path.display()))?;

    let table: toml::Value = toml::from_str(&text)
        .with_context(|| format!("failed to parse {}", cargo_toml_path.display()))?;

    let features_tbl = table
        .get("features")
        .and_then(toml::Value::as_table)
        .context("[features] section missing in Cargo.toml")?;

    let mut all: Vec<String> = features_tbl
        .keys()
        .filter(|name| name.as_str() != "default")
        .cloned()
        .collect();
    all.sort();

    let default = features_tbl
        .get("default")
        .and_then(toml::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(toml::Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let kernel_all = all
        .iter()
        .filter(|name| is_kernel_feature_name(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    let kernel_default = default
        .iter()
        .filter(|name| is_kernel_feature_name(name.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    Ok(FeatureCatalog {
        all,
        default,
        kernel_all,
        kernel_default,
    })
}

pub fn kernel_features_from_default(required: &[&str]) -> Result<KernelFeatures> {
    let catalog = load_feature_catalog()?;
    let mut names = catalog.kernel_default;
    add_required(&mut names, required);
    Ok(kernel_features_from_names(&names))
}

pub fn kernel_features_from_all(required: &[&str]) -> Result<KernelFeatures> {
    let catalog = load_feature_catalog()?;
    let mut names = catalog.kernel_all;
    add_required(&mut names, required);
    Ok(kernel_features_from_names(&names))
}

pub fn prompt_kernel_feature_selection(
    purpose: &str,
    required: &[&str],
) -> Result<KernelFeatures> {
    let catalog = load_feature_catalog()?;

    if catalog.kernel_all.is_empty() {
        logging::warn(
            "features",
            "No kernel-compatible features found in Cargo.toml; falling back to required set",
            &[("purpose", purpose)],
        );
        let mut names = Vec::new();
        add_required(&mut names, required);
        return Ok(kernel_features_from_names(&names));
    }

    let mode_options = [
        KernelFeatureSelectionMode::Default,
        KernelFeatureSelectionMode::All,
        KernelFeatureSelectionMode::Custom,
    ];
    let mode = *ui::select(
        &format!("{}: choose kernel feature mode", purpose),
        &mode_options,
    )?;

    match mode {
        KernelFeatureSelectionMode::Default => {
            let mut names = catalog.kernel_default;
            add_required(&mut names, required);
            Ok(kernel_features_from_names(&names))
        }
        KernelFeatureSelectionMode::All => {
            let mut names = catalog.kernel_all;
            add_required(&mut names, required);
            Ok(kernel_features_from_names(&names))
        }
        KernelFeatureSelectionMode::Custom => {
            let mut defaults = catalog.kernel_default.clone();
            add_required(&mut defaults, required);

            let default_idx = catalog
                .kernel_all
                .iter()
                .enumerate()
                .filter_map(|(idx, name)| defaults.iter().any(|d| d == name).then_some(idx))
                .collect::<Vec<_>>();

            let selected_idx = ui::multiselect(
                &format!("{}: select kernel features", purpose),
                &catalog.kernel_all,
                &default_idx,
            )?;

            let mut selected = selected_idx
                .into_iter()
                .filter_map(|idx| catalog.kernel_all.get(idx).cloned())
                .collect::<Vec<_>>();
            add_required(&mut selected, required);
            Ok(kernel_features_from_names(&selected))
        }
    }
}

pub fn test_feature_csv() -> String {
    let required = ["kernel_test_mode", "vfs", "drivers"];

    match load_feature_catalog() {
        Ok(catalog) => {
            let mut names = catalog.default;
            add_required(&mut names, &required);
            unique_sorted(&mut names);
            names.join(",")
        }
        Err(_) => required.join(","),
    }
}

pub fn cargo_features_from_default(required: &[&str]) -> Result<String> {
    let catalog = load_feature_catalog()?;
    let mut names = catalog.default;
    add_required(&mut names, required);
    Ok(names.join(","))
}

pub fn cargo_features_from_all(required: &[&str]) -> Result<String> {
    let catalog = load_feature_catalog()?;
    let mut names = catalog.all;
    add_required(&mut names, required);
    Ok(names.join(","))
}

pub fn prompt_cargo_feature_selection(purpose: &str, required: &[&str]) -> Result<String> {
    let catalog = load_feature_catalog()?;

    if catalog.all.is_empty() {
        let mut names = Vec::new();
        add_required(&mut names, required);
        return Ok(names.join(","));
    }

    let mode_options = [
        CargoFeatureSelectionMode::Default,
        CargoFeatureSelectionMode::All,
        CargoFeatureSelectionMode::Custom,
    ];
    let mode = *ui::select(
        &format!("{}: choose cargo feature mode", purpose),
        &mode_options,
    )?;

    match mode {
        CargoFeatureSelectionMode::Default => {
            let mut names = catalog.default;
            add_required(&mut names, required);
            Ok(names.join(","))
        }
        CargoFeatureSelectionMode::All => {
            let mut names = catalog.all;
            add_required(&mut names, required);
            Ok(names.join(","))
        }
        CargoFeatureSelectionMode::Custom => {
            let mut defaults = catalog.default.clone();
            add_required(&mut defaults, required);

            let default_idx = catalog
                .all
                .iter()
                .enumerate()
                .filter_map(|(idx, name)| defaults.iter().any(|d| d == name).then_some(idx))
                .collect::<Vec<_>>();

            let selected_idx = ui::multiselect(
                &format!("{}: select cargo features", purpose),
                &catalog.all,
                &default_idx,
            )?;

            let mut selected = selected_idx
                .into_iter()
                .filter_map(|idx| catalog.all.get(idx).cloned())
                .collect::<Vec<_>>();
            add_required(&mut selected, required);
            Ok(selected.join(","))
        }
    }
}

fn is_kernel_feature_name(name: &str) -> bool {
    !KernelFeatures::from_cargo_features(&[name]).is_empty()
}

fn kernel_features_from_names(names: &[String]) -> KernelFeatures {
    let refs = names.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    KernelFeatures::from_cargo_features(&refs)
}

fn add_required(names: &mut Vec<String>, required: &[&str]) {
    for req in required {
        let req_name = req.to_string();
        if !names.iter().any(|name| name == &req_name) {
            names.push(req_name);
        }
    }
    unique_sorted(names);
}

fn unique_sorted(names: &mut Vec<String>) {
    let mut set = BTreeSet::new();
    for name in names.drain(..) {
        set.insert(name);
    }
    names.extend(set);
}
