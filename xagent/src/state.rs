use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use chrono::{DateTime, Utc};
use crate::config::AgentConfig;
use crate::models::{
    Action, AgentEvent, Confirmation, Host, IdempotencyRecord, Job, JobStatus, PolicyTrace, RecentEntry, RolePolicy,
    ScheduledTask, default_actions, default_role_policies,
};

const MAX_RECENT: usize = 200;
const MAX_POLICY_TRACES: usize = 500;
const MAX_EVENTS: usize = 1000;
const MAX_EVENTS_BOOTSTRAP: usize = 200;

// ── Token / role state ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TokenRegistry {
    /// role → token
    pub tokens: HashMap<String, String>,
    /// role → updated_utc
    pub updated: HashMap<String, Option<DateTime<Utc>>>,
}

impl TokenRegistry {
    pub fn from_config(cfg: &AgentConfig) -> Self {
        let mut tokens = HashMap::new();
        let mut updated: HashMap<String, Option<DateTime<Utc>>> = HashMap::new();

        let admin_tok = if cfg.tokens.admin.is_empty() {
            cfg.auth_token.clone()
        } else {
            cfg.tokens.admin.clone()
        };

        tokens.insert("viewer".into(), cfg.tokens.viewer.clone());
        tokens.insert("operator".into(), cfg.tokens.operator.clone());
        tokens.insert("admin".into(), admin_tok);

        updated.insert("viewer".into(), None);
        updated.insert("operator".into(), None);
        updated.insert("admin".into(), Some(Utc::now()));

        TokenRegistry { tokens, updated }
    }

    pub fn resolve_role(&self, token: &str) -> Option<&str> {
        if token.is_empty() {
            return None;
        }
        // Highest privilege first
        for role in ["admin", "operator", "viewer"] {
            if let Some(t) = self.tokens.get(role) {
                if !t.is_empty() && t == token {
                    return Some(role);
                }
            }
        }
        None
    }
}

// ── Inner mutable state ───────────────────────────────────────────────────────

pub struct Inner {
    pub started_utc: DateTime<Utc>,
    pub tokens: TokenRegistry,
    pub unsafe_no_auth: bool,
    pub max_concurrency: u32,
    pub max_queue: u32,
    pub allowed_origins: Vec<String>,
    pub actions: Vec<Action>,
    pub jobs: HashMap<String, Job>,
    pub queue: Vec<String>,       // job IDs ordered by priority
    pub recent: Vec<RecentEntry>, // capped ring
    pub scheduler_enabled: bool,
    pub schedules: HashMap<String, ScheduledTask>,
    pub confirmations: HashMap<String, Confirmation>,
    pub hosts: Vec<Host>,
    pub role_policies: HashMap<String, RolePolicy>,
    pub policy_traces: Vec<PolicyTrace>,
    pub events: Vec<AgentEvent>,
    pub idempotency: HashMap<String, IdempotencyRecord>,
    pub log_retention_days: u32,
    pub audit_dir: String,
}

impl Inner {
    pub fn from_config(cfg: &AgentConfig) -> Self {
        let tokens = TokenRegistry::from_config(cfg);

        // Build initial scheduler state
        let mut schedules = HashMap::new();
        let now = Utc::now();
        for t in &cfg.scheduler.tasks {
            let interval = t.interval_sec.max(60);
            let id = t.id.clone();
            schedules.insert(
                id.clone(),
                ScheduledTask {
                    id,
                    action: t.action.clone(),
                    interval_sec: interval,
                    priority: t.priority.clone(),
                    enabled: t.enabled,
                    last_run_utc: None,
                    next_run_utc: now + chrono::Duration::seconds(interval as i64),
                    source: format!("scheduler:{}", t.id),
                },
            );
        }

        // Build role policies
        let mut role_policies = default_role_policies();
        for (role, rp) in &cfg.policy.roles {
            if let Some(entry) = role_policies.get_mut(role) {
                if !rp.max_risk.is_empty() {
                    entry.max_risk = rp.max_risk.clone();
                }
                if !rp.denied_actions.is_empty() {
                    entry.denied_actions = rp.denied_actions.clone();
                }
                if !rp.denied_categories.is_empty() {
                    entry.denied_categories = rp.denied_categories.clone();
                }
            }
        }

        // Build hosts (always include local)
        let mut hosts: Vec<Host> = vec![Host {
            id: "local".into(),
            name: "Localhost".into(),
            url: format!("http://127.0.0.1:{}", cfg.port),
            enabled: true,
            role_hint: "admin".into(),
            token: String::new(),
            last_seen_utc: Some(now),
            reachable: Some(true),
            capabilities: vec!["local-executor".into(), "admin".into()],
        }];
        for h in &cfg.hosts {
            if h.id == "local" {
                continue;
            }
            hosts.push(Host {
                id: h.id.clone(),
                name: h.name.clone().unwrap_or_else(|| h.id.clone()),
                url: h.url.clone(),
                enabled: h.enabled,
                role_hint: if h.role_hint.is_empty() { "operator".into() } else { h.role_hint.clone() },
                token: h.token.clone(),
                last_seen_utc: None,
                reachable: None,
                capabilities: vec![],
            });
        }

        Inner {
            started_utc: now,
            tokens,
            unsafe_no_auth: cfg.auth_mode.to_lowercase() == "unsafe",
            max_concurrency: cfg.max_concurrency.max(1),
            max_queue: cfg.max_queue.max(10),
            allowed_origins: cfg.allowed_origins.clone(),
            actions: default_actions(),
            jobs: HashMap::new(),
            queue: Vec::new(),
            recent: Vec::new(),
            scheduler_enabled: cfg.scheduler.enabled,
            schedules,
            confirmations: HashMap::new(),
            hosts,
            role_policies,
            policy_traces: Vec::new(),
            events: load_recent_events("reports/tooling/agent_runs/events.jsonl"),
            idempotency: HashMap::new(),
            log_retention_days: cfg.log_retention_days.max(1),
            audit_dir: "reports/tooling/agent_runs".into(),
        }
    }

    // ── Job helpers ───────────────────────────────────────────────────────────

    pub fn running_count(&self) -> usize {
        self.jobs.values().filter(|j| j.status == JobStatus::Running).count()
    }

    pub fn queue_count(&self) -> usize {
        self.queue.len()
    }

    pub fn is_busy(&self) -> bool {
        self.running_count() as u32 >= self.max_concurrency
    }

    pub fn push_recent(&mut self, entry: RecentEntry) {
        if self.recent.len() >= MAX_RECENT {
            self.recent.remove(0);
        }
        self.recent.push(entry);
    }

    pub fn push_policy_trace(&mut self, trace: PolicyTrace) {
        if self.policy_traces.len() >= MAX_POLICY_TRACES {
            self.policy_traces.remove(0);
        }
        self.policy_traces.push(trace);
    }

    pub fn push_event(&mut self, event: AgentEvent) {
        if self.events.len() >= MAX_EVENTS {
            self.events.remove(0);
        }
        self.events.push(event);
        if let Some(latest) = self.events.last() {
            persist_event_jsonl(&self.audit_dir, latest);
        }
    }

    pub fn action_by_id(&self, id: &str) -> Option<&Action> {
        self.actions.iter().find(|a| a.id == id)
    }

    // ── Auth helpers ──────────────────────────────────────────────────────────

    pub fn resolve_role(&self, token: &str) -> &str {
        if let Some(role) = self.tokens.resolve_role(token) {
            return role;
        }
        "anonymous"
    }

    /// Returns true when the given role meets the minimum required level.
    pub fn role_at_least(role: &str, required: &str) -> bool {
        let order = ["anonymous", "viewer", "operator", "admin"];
        let ri = order.iter().position(|r| *r == role).unwrap_or(0);
        let rq = order.iter().position(|r| *r == required).unwrap_or(0);
        ri >= rq
    }

    // ── Policy check ──────────────────────────────────────────────────────────

    /// Returns Ok(()) if the role is permitted to run the action, Err(reason) otherwise.
    pub fn check_policy(&self, role: &str, action_id: &str) -> Result<(), String> {
        let policy = match self.role_policies.get(role) {
            Some(p) => p,
            None => return Err("no_policy".into()),
        };
        // denied_actions glob "*" blocks all
        if policy.denied_actions.iter().any(|d| d == "*") {
            return Err("denied_all".into());
        }
        if policy.denied_actions.iter().any(|d| d == action_id) {
            return Err("denied_action".into());
        }
        if let Some(action) = self.action_by_id(action_id) {
            if policy.denied_categories.iter().any(|c| c == &action.category) {
                return Err("denied_category".into());
            }
            let action_risk = crate::models::Risk::from_str(&action.risk);
            let max_risk = crate::models::Risk::from_str(&policy.max_risk);
            if action_risk > max_risk {
                return Err("risk_too_high".into());
            }
        }
        Ok(())
    }
}

pub fn prune_expired_confirmations(state: &AppState) -> usize {
    let now = Utc::now();
    let mut inner = state.write();
    let before = inner.confirmations.len();
    inner.confirmations.retain(|_, c| c.expires_utc > now);
    before.saturating_sub(inner.confirmations.len())
}

pub fn prune_idempotency_registry(state: &AppState, ttl_hours: i64) -> usize {
    let threshold = Utc::now() - chrono::Duration::hours(ttl_hours.max(1));
    let mut inner = state.write();
    let before = inner.idempotency.len();
    inner.idempotency.retain(|_, record| record.created_utc > threshold);
    before.saturating_sub(inner.idempotency.len())
}

fn persist_event_jsonl(audit_dir: &str, event: &AgentEvent) {
    let dir = std::path::Path::new(audit_dir);
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let file_path = dir.join("events.jsonl");
    let mut file = match std::fs::OpenOptions::new().create(true).append(true).open(&file_path) {
        Ok(file) => file,
        Err(_) => return,
    };
    if let Ok(mut line) = serde_json::to_string(event) {
        line.push('\n');
        let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    }
}

fn load_recent_events(path: &str) -> Vec<AgentEvent> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in content.lines().rev().take(MAX_EVENTS_BOOTSTRAP).collect::<Vec<_>>().into_iter().rev() {
        if let Ok(event) = serde_json::from_str::<AgentEvent>(line) {
            out.push(event);
        }
    }
    out
}

// ── AppState (shared via Rocket managed state) ────────────────────────────────

#[derive(Clone)]
pub struct AppState(pub Arc<RwLock<Inner>>);

impl AppState {
    pub fn new(cfg: &AgentConfig) -> Self {
        AppState(Arc::new(RwLock::new(Inner::from_config(cfg))))
    }

    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, Inner> {
        self.0.read().expect("state lock poisoned")
    }

    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, Inner> {
        self.0.write().expect("state lock poisoned")
    }
}

// ── Scheduler tick (called periodically from the background task) ─────────────

/// Checks schedules and enqueues any that are due. Returns IDs of triggered tasks.
pub fn tick_scheduler(state: &AppState) -> Vec<String> {
    let now = Utc::now();
    let due_ids: Vec<String> = {
        let inner = state.read();
        if !inner.scheduler_enabled {
            return vec![];
        }
        inner
            .schedules
            .values()
            .filter(|s| s.enabled && now >= s.next_run_utc)
            .map(|s| s.id.clone())
            .collect()
    };

    let mut triggered = vec![];
    for task_id in due_ids {
        let (action, priority, source, interval) = {
            let inner = state.read();
            let s = match inner.schedules.get(&task_id) {
                Some(s) => s,
                None => continue,
            };
            // Verify action exists
            if inner.action_by_id(&s.action).is_none() {
                continue;
            }
            (s.action.clone(), s.priority.clone(), s.source.clone(), s.interval_sec)
        };

        // Enqueue job
        crate::actions::enqueue_job(state, &action, &priority, &source);

        // Update schedule
        let mut inner = state.write();
        if let Some(s) = inner.schedules.get_mut(&task_id) {
            s.last_run_utc = Some(now);
            s.next_run_utc = now + chrono::Duration::seconds(interval as i64);
        }
        triggered.push(task_id);
    }
    triggered
}
