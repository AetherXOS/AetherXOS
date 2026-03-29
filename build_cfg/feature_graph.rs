//! Feature dependency graph scanning from Cargo.toml [features].

use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;

pub fn feature_enabled(name: &str) -> bool {
    let key = format!(
        "CARGO_FEATURE_{}",
        name.to_ascii_uppercase().replace('-', "_")
    );
    env::var_os(key).is_some()
}

pub fn normalize_feature_ref(raw: &str) -> Option<String> {
    let mut s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix("dep:") {
        if rest.is_empty() {
            return None;
        }
        return Some(rest.to_string());
    }
    if let Some((left, _)) = s.split_once('/') {
        s = left;
    }
    if let Some(trimmed) = s.strip_suffix('?') {
        s = trimmed;
    }
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

pub fn load_feature_graph_from_manifest() -> BTreeMap<String, Vec<String>> {
    let raw = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml for feature scan");
    let value: toml::Value =
        toml::from_str(&raw).expect("Failed to parse Cargo.toml for feature scan");
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    if let Some(feature_tbl) = value.get("features").and_then(|v| v.as_table()) {
        for (k, v) in feature_tbl {
            let mut deps = Vec::new();
            match v {
                toml::Value::Array(items) => {
                    for item in items {
                        if let Some(s) = item.as_str() {
                            if let Some(dep) = normalize_feature_ref(s) {
                                deps.push(dep);
                            }
                        }
                    }
                }
                toml::Value::String(s) => {
                    if let Some(dep) = normalize_feature_ref(s) {
                        deps.push(dep);
                    }
                }
                _ => {}
            }
            deps.sort();
            deps.dedup();
            graph.insert(k.to_string(), deps);
        }
    }
    graph
}

fn collect_reachable_roots(
    feature: &str,
    graph: &BTreeMap<String, Vec<String>>,
    roots: &HashSet<String>,
    out: &mut HashSet<String>,
    visiting: &mut HashSet<String>,
) {
    if !visiting.insert(feature.to_string()) {
        return;
    }
    if roots.contains(feature) {
        out.insert(feature.to_string());
    }
    if let Some(deps) = graph.get(feature) {
        for dep in deps {
            if graph.contains_key(dep) {
                collect_reachable_roots(dep, graph, roots, out, visiting);
            }
        }
    }
}

pub fn feature_primary_group(feature: &str, graph: &BTreeMap<String, Vec<String>>) -> String {
    let mut roots = HashSet::new();
    for (name, deps) in graph {
        if deps.is_empty() {
            roots.insert(name.clone());
        }
    }
    let mut reachable = HashSet::new();
    let mut visiting = HashSet::new();
    collect_reachable_roots(feature, graph, &roots, &mut reachable, &mut visiting);
    let mut candidates: Vec<String> = reachable.into_iter().collect();
    candidates.sort();
    if let Some(first) = candidates.first() {
        return first.clone();
    }
    if let Some((prefix, _)) = feature.split_once('_') {
        return prefix.to_string();
    }
    feature.to_string()
}
