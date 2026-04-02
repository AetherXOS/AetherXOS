use crate::models::ConfigSnapshot;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn metadata_table<'a>(
    root: &'a toml::Value,
    path: &[&str],
) -> Option<&'a toml::map::Map<String, toml::Value>> {
    let mut current = root;
    for part in path {
        current = current.get(*part)?;
    }
    current.as_table()
}

fn stringify_table(table: &toml::map::Map<String, toml::Value>) -> BTreeMap<String, String> {
    table
        .iter()
        .map(|(key, value)| {
            let rendered = match value {
                toml::Value::String(v) => v.clone(),
                _ => value.to_string(),
            };
            (key.clone(), rendered)
        })
        .collect()
}

pub fn load_config_snapshot(repo_root: &Path) -> Result<ConfigSnapshot, String> {
    let cargo_toml = repo_root.join("Cargo.toml");
    let text = fs::read_to_string(&cargo_toml)
        .map_err(|err| format!("failed to read {}: {err}", cargo_toml.display()))?;
    let value: toml::Value = toml::from_str(&text)
        .map_err(|err| format!("failed to parse {}: {err}", cargo_toml.display()))?;

    let linux_compat = metadata_table(
        &value,
        &["package", "metadata", "aethercore", "config", "linux_compat"],
    )
    .map(stringify_table)
    .unwrap_or_default();
    let linux_os = metadata_table(
        &value,
        &["package", "metadata", "aethercore", "config", "linux_os"],
    )
    .map(stringify_table)
    .unwrap_or_default();

    Ok(ConfigSnapshot {
        linux_compat,
        linux_os,
    })
}
