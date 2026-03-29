use super::{ConfigKeySpec, KernelConfig};
use crate::generated_consts::{
    CARGO_FEATURE_ENABLED, CARGO_FEATURE_NAMES, CARGO_FEATURE_PRIMARY_GROUP, LIBNET_L2_ENABLED,
    LIBNET_L34_ENABLED, LIBNET_L6_ENABLED, LIBNET_L7_ENABLED,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileTimeFeatureView {
    pub name: &'static str,
    pub enabled: bool,
    pub category: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LibraryCompileProfile {
    pub expose_vfs_api: bool,
    pub expose_network_api: bool,
    pub expose_ipc_api: bool,
    pub expose_proc_config_api: bool,
    pub expose_sysctl_api: bool,
    pub libnet_l2: bool,
    pub libnet_l34: bool,
    pub libnet_l6: bool,
    pub libnet_l7: bool,
}

impl KernelConfig {
    pub fn cargo_feature_names() -> &'static [&'static str] {
        &CARGO_FEATURE_NAMES
    }

    pub fn is_cargo_feature_enabled(name: &str) -> Option<bool> {
        let mut idx = 0usize;
        while idx < CARGO_FEATURE_NAMES.len() {
            if CARGO_FEATURE_NAMES[idx] == name {
                return Some(CARGO_FEATURE_ENABLED[idx]);
            }
            idx += 1;
        }
        None
    }

    pub fn runtime_config_keys() -> &'static [ConfigKeySpec] {
        Self::runtime_config_catalog()
    }

    pub fn cargo_feature_category(name: &str) -> &'static str {
        let mut idx = 0usize;
        while idx < CARGO_FEATURE_NAMES.len() {
            if CARGO_FEATURE_NAMES[idx] == name {
                let canonical = CARGO_FEATURE_NAMES[idx];
                let primary = CARGO_FEATURE_PRIMARY_GROUP[idx];
                if canonical != "schedulers" && primary == "schedulers" {
                    return "scheduler";
                }
                if canonical == "process_abstraction" || canonical == "default" {
                    return "core";
                }
                if !primary.is_empty() {
                    return primary;
                }
                return canonical;
            }
            idx += 1;
        }
        "other"
    }

    pub fn visit_cargo_feature_catalog<F>(mut visit: F)
    where
        F: FnMut(CompileTimeFeatureView),
    {
        let mut i = 0usize;
        while i < CARGO_FEATURE_NAMES.len() {
            let name = CARGO_FEATURE_NAMES[i];
            let enabled = CARGO_FEATURE_ENABLED[i];
            visit(CompileTimeFeatureView {
                name,
                enabled,
                category: Self::cargo_feature_category(name),
            });
            i += 1;
        }
    }

    pub fn scheduler_feature_names() -> &'static [&'static str] {
        &CARGO_FEATURE_NAMES
    }

    pub fn is_scheduler_feature(name: &str) -> bool {
        let mut i = 0usize;
        while i < CARGO_FEATURE_NAMES.len() {
            if CARGO_FEATURE_NAMES[i] == name {
                return CARGO_FEATURE_NAMES[i] != "schedulers"
                    && CARGO_FEATURE_PRIMARY_GROUP[i] == "schedulers";
            }
            i += 1;
        }
        false
    }

    pub fn visit_scheduler_features<F>(mut visit: F)
    where
        F: FnMut(CompileTimeFeatureView),
    {
        let mut i = 0usize;
        while i < CARGO_FEATURE_NAMES.len() {
            if CARGO_FEATURE_NAMES[i] != "schedulers"
                && CARGO_FEATURE_PRIMARY_GROUP[i] == "schedulers"
            {
                visit(CompileTimeFeatureView {
                    name: CARGO_FEATURE_NAMES[i],
                    enabled: CARGO_FEATURE_ENABLED[i],
                    category: "scheduler",
                });
            }
            i += 1;
        }
    }

    pub fn enabled_scheduler_feature_count() -> usize {
        let mut count = 0usize;
        let mut i = 0usize;
        while i < CARGO_FEATURE_NAMES.len() {
            if CARGO_FEATURE_NAMES[i] != "schedulers"
                && CARGO_FEATURE_PRIMARY_GROUP[i] == "schedulers"
                && CARGO_FEATURE_ENABLED[i]
            {
                count += 1;
            }
            i += 1;
        }
        count
    }

    pub fn enabled_scheduler_feature_at(index: usize) -> Option<&'static str> {
        let mut hit = 0usize;
        let mut i = 0usize;
        while i < CARGO_FEATURE_NAMES.len() {
            let feature = CARGO_FEATURE_NAMES[i];
            if feature != "schedulers"
                && CARGO_FEATURE_PRIMARY_GROUP[i] == "schedulers"
                && CARGO_FEATURE_ENABLED[i]
            {
                if hit == index {
                    return Some(feature);
                }
                hit += 1;
            }
            i += 1;
        }
        None
    }

    pub fn primary_scheduler_feature() -> Option<&'static str> {
        Self::enabled_scheduler_feature_at(0)
    }

    pub fn library_compile_profile() -> LibraryCompileProfile {
        let cargo_profile = Self::library_cargo_feature_profile();
        LibraryCompileProfile {
            expose_vfs_api: cargo_profile.expose_vfs_api,
            expose_network_api: cargo_profile.expose_network_api,
            expose_ipc_api: cargo_profile.expose_ipc_api,
            expose_proc_config_api: cargo_profile.expose_proc_config_api,
            expose_sysctl_api: cargo_profile.expose_sysctl_api,
            libnet_l2: LIBNET_L2_ENABLED,
            libnet_l34: LIBNET_L34_ENABLED,
            libnet_l6: LIBNET_L6_ENABLED,
            libnet_l7: LIBNET_L7_ENABLED,
        }
    }
}
