//! Scans kernel/src/config/ for runtime setters and generates the key catalog.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct RuntimeSetterSpec {
    stem: String,
    fn_name: String,
    value_kind: String,
    apply_fn: String,
    raw_type: String,
}

fn list_rs_files(root: &Path, out: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                list_rs_files(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(path.to_string_lossy().to_string());
            }
        }
    }
}

fn map_runtime_setter_type(raw_ty: &str) -> Option<(&'static str, &'static str)> {
    match raw_ty.trim() {
        "bool" => Some(("Bool", "set_bool")),
        "u8" => Some(("U8", "set_u8")),
        "u16" => Some(("U16", "set_u16")),
        "u32" => Some(("U32", "set_u32")),
        "u64" => Some(("U64", "set_u64")),
        "usize" => Some(("Usize", "set_usize")),
        "TlsPolicyProfile" => Some(("TlsPolicy", "set_tls")),
        "BoundaryMode" => Some(("BoundaryMode", "set_boundary_mode")),
        "DevFsPolicyProfile" => Some(("DevFsPolicy", "set_devfs_policy")),
        "VirtualizationExecutionProfile" => {
            Some(("VirtualizationExecution", "set_virtualization_execution"))
        }
        "VirtualizationGovernorProfile" => {
            Some(("VirtualizationGovernor", "set_virtualization_governor"))
        }
        _ => None,
    }
}

fn enum_parser_type_for_kind(kind: &str, raw_ty: &str) -> Option<String> {
    match kind {
        "VirtualizationExecution" => Some("VirtualizationExecutionClass".to_string()),
        "VirtualizationGovernor" => Some("VirtualizationGovernorClass".to_string()),
        "TlsPolicy" | "BoundaryMode" | "DevFsPolicy" => Some(raw_ty.to_string()),
        _ => None,
    }
}

fn scan_runtime_setters() -> Vec<RuntimeSetterSpec> {
    let mut files = vec!["kernel/src/config.rs".to_string()];
    list_rs_files(Path::new("kernel/src/config"), &mut files);

    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for file in files {
        let raw = match fs::read_to_string(&file) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for line in raw.lines() {
            let t = line.trim();
            if !t.starts_with("pub fn set_") {
                continue;
            }
            let name_end = match t.find('(') {
                Some(i) => i,
                None => continue,
            };
            let fn_name = t["pub fn ".len()..name_end].trim().to_string();
            if !fn_name.starts_with("set_") {
                continue;
            }
            let sig = &t[name_end + 1..];
            let marker = "value: Option<";
            let start = match sig.find(marker) {
                Some(i) => i + marker.len(),
                None => continue,
            };
            let tail = &sig[start..];
            let end = match tail.find('>') {
                Some(i) => i,
                None => continue,
            };
            let ty = tail[..end].trim();
            let (value_kind, apply_fn) = match map_runtime_setter_type(ty) {
                Some(v) => v,
                None => continue,
            };
            let stem = fn_name.trim_start_matches("set_").to_string();
            if !seen.insert(stem.clone()) {
                continue;
            }
            out.push(RuntimeSetterSpec {
                stem,
                fn_name,
                value_kind: value_kind.to_string(),
                apply_fn: apply_fn.to_string(),
                raw_type: ty.to_string(),
            });
        }
    }

    out.sort_by(|a, b| a.stem.cmp(&b.stem));
    out
}

fn scan_enum_variants() -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    let raw = match fs::read_to_string("kernel/src/config/parsers.rs") {
        Ok(v) => v,
        Err(_) => return out,
    };
    let lines: Vec<&str> = raw.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("pub enum ") && t.ends_with('{') {
            let name = t
                .trim_start_matches("pub enum ")
                .trim_end_matches('{')
                .trim()
                .to_string();
            let mut vars = Vec::new();
            i += 1;
            while i < lines.len() {
                let v = lines[i].trim();
                if v.starts_with('}') {
                    break;
                }
                if v.is_empty() || v.starts_with("//") {
                    i += 1;
                    continue;
                }
                if let Some((left, _)) = v.split_once(',') {
                    let token = left.trim();
                    if !token.is_empty()
                        && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        vars.push(token.to_string());
                    }
                }
                i += 1;
            }
            vars.sort();
            vars.dedup();
            out.insert(name, vars);
        }
        i += 1;
    }
    out
}

pub fn generate_runtime_key_autogen() {
    let specs = scan_runtime_setters();
    let enum_variants = scan_enum_variants();
    let mut cats: Vec<String> = specs
        .iter()
        .map(|s| s.stem.split('_').next().unwrap_or("other").to_string())
        .collect();
    cats.sort();
    cats.dedup();
    let mut content = String::new();
    content.push_str("//! AUTO-GENERATED RUNTIME CONFIG KEY CATALOG - DO NOT EDIT\n");
    content.push_str(
        "//! Generated by build.rs from set_* Option<T> setters under kernel/src/config*\n\n",
    );
    content.push_str("use super::{ConfigKeySpec, ConfigSetError, ConfigValue, ConfigValueKind, KernelConfig};\n\n");
    content.push_str("pub(super) const AUTO_RUNTIME_CONFIG_CATEGORIES: &[&str] = &[");
    content.push_str(
        &cats
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", "),
    );
    content.push_str("];\n\n");
    content.push_str("pub(super) const AUTO_RUNTIME_CONFIG_KEYS: &[ConfigKeySpec] = &[\n");
    for s in &specs {
        content.push_str(&format!(
            "    ConfigKeySpec {{ key: \"{}\", value_kind: ConfigValueKind::{}, description: \"auto:{}\" }},\n",
            s.stem, s.value_kind, s.fn_name
        ));
    }
    content.push_str("];\n\n");
    content.push_str("pub(super) fn auto_set_by_stem(stem: &str, value: Option<ConfigValue>) -> Result<(), ConfigSetError> {\n");
    content.push_str("    match stem {\n");
    for s in &specs {
        content.push_str(&format!(
            "        \"{}\" => super::{}(value, KernelConfig::{}),\n",
            s.stem, s.apply_fn, s.fn_name
        ));
    }
    content.push_str("        _ => Err(ConfigSetError::UnknownKey),\n");
    content.push_str("    }\n");
    content.push_str("}\n");

    content.push_str("\npub(super) fn auto_parse_typed_value(kind: ConfigValueKind, raw: &str) -> Result<ConfigValue, ConfigSetError> {\n");
    content.push_str("    match kind {\n");
    content.push_str(
        "        ConfigValueKind::Bool => super::parse_bool(raw).map(ConfigValue::Bool),\n",
    );
    content.push_str("        ConfigValueKind::U8 => super::parse_u8(raw).map(ConfigValue::U8),\n");
    content
        .push_str("        ConfigValueKind::U16 => super::parse_u16(raw).map(ConfigValue::U16),\n");
    content
        .push_str("        ConfigValueKind::U32 => super::parse_u32(raw).map(ConfigValue::U32),\n");
    content
        .push_str("        ConfigValueKind::U64 => super::parse_u64(raw).map(ConfigValue::U64),\n");
    content.push_str(
        "        ConfigValueKind::Usize => super::parse_usize(raw).map(ConfigValue::Usize),\n",
    );
    let mut enum_kind_to_type = BTreeMap::<String, String>::new();
    for s in &specs {
        if let Some(parser_ty) = enum_parser_type_for_kind(s.value_kind.as_str(), &s.raw_type) {
            enum_kind_to_type.insert(s.value_kind.clone(), parser_ty);
        }
    }
    for (kind, ty) in enum_kind_to_type {
        let vars = enum_variants.get(&ty).cloned().unwrap_or_default();
        content.push_str(&format!("        ConfigValueKind::{} => {{\n", kind));
        if !vars.is_empty() {
            content.push_str("            let mut valid = false;\n");
            for v in vars {
                content.push_str(&format!(
                    "            if raw.eq_ignore_ascii_case(\"{}\") {{ valid = true; }}\n",
                    v
                ));
            }
            content
                .push_str("            if !valid { return Err(ConfigSetError::InvalidValue); }\n");
        }
        match kind.as_str() {
            "TlsPolicy" => content.push_str(&format!(
                "            Ok(ConfigValue::TlsPolicy(super::{}::from_str(raw)))\n",
                ty
            )),
            "BoundaryMode" => content.push_str(&format!(
                "            Ok(ConfigValue::BoundaryMode(super::{}::from_str(raw)))\n",
                ty
            )),
            "DevFsPolicy" => content.push_str(&format!(
                "            Ok(ConfigValue::DevFsPolicy(super::{}::from_str(raw)))\n",
                ty
            )),
            "VirtualizationExecution" => content.push_str(&format!(
                "            Ok(ConfigValue::VirtualizationExecution(super::{}::from_str(raw)))\n",
                ty
            )),
            "VirtualizationGovernor" => content.push_str(&format!(
                "            Ok(ConfigValue::VirtualizationGovernor(super::{}::from_str(raw)))\n",
                ty
            )),
            _ => content.push_str("            Err(ConfigSetError::InvalidValue)\n"),
        }
        content.push_str("        }\n");
    }
    content.push_str("    }\n");
    content.push_str("}\n");

    fs::write("kernel/src/config/runtime_key_autogen.rs", content)
        .expect("Failed to write kernel/src/config/runtime_key_autogen.rs");
}
