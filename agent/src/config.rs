use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Loaded from scripts/config/hypercore.defaults.json → .agent section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_auth_mode")]
    pub auth_mode: String,
    #[serde(default = "default_token")]
    pub auth_token: String,
    #[serde(default)]
    pub tokens: TokenConfig,
    #[serde(default = "default_origins")]
    pub allowed_origins: Vec<String>,
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: u32,
    #[serde(default = "default_max_queue")]
    pub max_queue: u32,
    #[serde(default = "default_log_retention")]
    pub log_retention_days: u32,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub policy: AgentPolicyConfig,
    #[serde(default)]
    pub hosts: Vec<HostConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenConfig {
    #[serde(default)]
    pub viewer: String,
    #[serde(default)]
    pub operator: String,
    #[serde(default)]
    pub admin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    #[serde(default = "bool_true")]
    pub enabled: bool,
    #[serde(default = "default_scheduler_tasks")]
    pub tasks: Vec<SchedulerTaskConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerTaskConfig {
    pub id: String,
    pub action: String,
    pub interval_sec: u64,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "bool_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPolicyConfig {
    #[serde(default)]
    pub roles: HashMap<String, RolePolicyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RolePolicyConfig {
    #[serde(default)]
    pub max_risk: String,
    #[serde(default)]
    pub denied_actions: Vec<String>,
    #[serde(default)]
    pub denied_categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    pub id: String,
    pub name: Option<String>,
    pub url: String,
    #[serde(default = "bool_true")]
    pub enabled: bool,
    #[serde(default)]
    pub role_hint: String,
    #[serde(default)]
    pub token: String,
}

/// Top-level hypercore.defaults.json shape (only the `agent` key matters here)
#[derive(Debug, Deserialize)]
struct HypercoreDefaults {
    #[serde(default)]
    agent: AgentConfig,
}

fn default_port() -> u16 {
    7401
}
fn default_auth_mode() -> String {
    "strict".into()
}
fn default_token() -> String {
    "hypercore-local-dev-token".into()
}
fn default_origins() -> Vec<String> {
    vec![
        "http://127.0.0.1".into(),
        "http://localhost".into(),
        "http://127.0.0.1:5173".into(),
        "http://localhost:5173".into(),
    ]
}
fn default_max_concurrency() -> u32 {
    1
}
fn default_max_queue() -> u32 {
    100
}
fn default_log_retention() -> u32 {
    14
}
fn bool_true() -> bool {
    true
}
fn default_priority() -> String {
    "low".into()
}
fn default_scheduler_tasks() -> Vec<SchedulerTaskConfig> {
    vec![
        SchedulerTaskConfig {
            id: "nightly_smoke".into(),
            action: "qemu_smoke".into(),
            interval_sec: 86400,
            priority: "low".into(),
            enabled: true,
        },
        SchedulerTaskConfig {
            id: "weekly_quality_gate".into(),
            action: "quality_gate".into(),
            interval_sec: 604800,
            priority: "low".into(),
            enabled: true,
        },
        SchedulerTaskConfig {
            id: "dashboard_refresh".into(),
            action: "dashboard_build".into(),
            interval_sec: 21600,
            priority: "low".into(),
            enabled: true,
        },
    ]
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            enabled: bool_true(),
            tasks: default_scheduler_tasks(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            auth_mode: default_auth_mode(),
            auth_token: default_token(),
            tokens: TokenConfig {
                viewer: String::new(),
                operator: String::new(),
                admin: default_token(),
            },
            allowed_origins: default_origins(),
            max_concurrency: default_max_concurrency(),
            max_queue: default_max_queue(),
            log_retention_days: default_log_retention(),
            scheduler: SchedulerConfig::default(),
            policy: AgentPolicyConfig::default(),
            hosts: Vec::new(),
        }
    }
}

/// Loads agent config from the given path (or falls back to defaults).
pub fn load_agent_config(path: Option<&str>) -> AgentConfig {
    let path = path.unwrap_or("scripts/config/hypercore.defaults.json");
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Could not read config file {path}: {e}. Using defaults.");
            return AgentConfig::default();
        }
    };
    match serde_json::from_str::<HypercoreDefaults>(&content) {
        Ok(root) => {
            let mut cfg = root.agent;
            // Fallback: if admin token is empty, use auth_token
            if cfg.tokens.admin.is_empty() {
                cfg.tokens.admin = cfg.auth_token.clone();
            }
            if cfg.allowed_origins.is_empty() {
                cfg.allowed_origins = default_origins();
            }
            if cfg.auth_token.is_empty() || cfg.auth_token == "change-me-hypercore-agent-token" {
                cfg.auth_token = "hypercore-local-dev-token".into();
            }
            cfg.max_concurrency = cfg.max_concurrency.max(1);
            cfg.max_queue = cfg.max_queue.max(10);
            cfg.log_retention_days = cfg.log_retention_days.max(1);
            cfg
        }
        Err(e) => {
            tracing::warn!("Failed to parse config {path}: {e}. Using defaults.");
            AgentConfig::default()
        }
    }
}
