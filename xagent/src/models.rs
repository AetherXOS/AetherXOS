use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

// ── Risk level ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Risk {
    Info,
    Med,
    High,
}

impl Risk {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "HIGH" => Risk::High,
            "MED" => Risk::Med,
            _ => Risk::Info,
        }
    }
}

// ── Action catalog ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub title: String,
    pub desc: String,
    /// The aethercore.ps1 sub-command name
    pub cmd: String,
    pub args: Vec<String>,
    pub risk: String,
    pub category: String,
    pub impact: String,
}

/// Returns the canonical action catalog (mirrors PowerShell $actions array).
pub fn default_actions() -> Vec<Action> {
    vec![
        Action {
            id: "doctor".into(),
            title: "Check Dependencies".into(),
            desc: "Readiness + version checks".into(),
            cmd: "doctor".into(),
            args: vec!["-WriteDoctorReport".into()],
            risk: "INFO".into(),
            category: "diagnostics".into(),
            impact: "Read-only host/tool checks.".into(),
        },
        Action {
            id: "doctor_fix".into(),
            title: "Install/Fix Dependencies".into(),
            desc: "Auto install missing tools".into(),
            cmd: "doctor-fix".into(),
            args: vec!["-AutoApprove".into()],
            risk: "HIGH".into(),
            category: "install".into(),
            impact: "Installs/changes host dependencies.".into(),
        },
        Action {
            id: "install_deno".into(),
            title: "Install Deno Runtime".into(),
            desc: "Install or repair Deno runtime on host".into(),
            cmd: "install-deno".into(),
            args: vec![],
            risk: "MED".into(),
            category: "install".into(),
            impact: "Installs Deno runtime via package manager.".into(),
        },
        Action {
            id: "build_iso".into(),
            title: "Build ISO".into(),
            desc: "Compile kernel and generate ISO".into(),
            cmd: "build-iso".into(),
            args: vec![],
            risk: "HIGH".into(),
            category: "build".into(),
            impact: "Writes boot artifacts and ISO outputs.".into(),
        },
        Action {
            id: "qemu_smoke".into(),
            title: "QEMU Smoke".into(),
            desc: "Automated boot smoke test".into(),
            cmd: "qemu-smoke".into(),
            args: vec![],
            risk: "HIGH".into(),
            category: "test".into(),
            impact: "Runs emulator smoke tests and writes reports.".into(),
        },
        Action {
            id: "qemu_live".into(),
            title: "QEMU Live".into(),
            desc: "Open interactive QEMU boot window".into(),
            cmd: "qemu-live".into(),
            args: vec![],
            risk: "MED".into(),
            category: "test".into(),
            impact: "Starts interactive emulator window.".into(),
        },
        Action {
            id: "dashboard_build".into(),
            title: "Build Dashboard UI".into(),
            desc: "Generate telemetry + build Svelte UI".into(),
            cmd: "os-smoke-dashboard".into(),
            args: vec![],
            risk: "MED".into(),
            category: "dashboard".into(),
            impact: "Rebuilds reports and dashboard assets.".into(),
        },
        Action {
            id: "dashboard_tests".into(),
            title: "Dashboard Tests".into(),
            desc: "Run unit + e2e tests".into(),
            cmd: "dashboard-ui-test".into(),
            args: vec![],
            risk: "MED".into(),
            category: "test".into(),
            impact: "Executes dashboard unit tests.".into(),
        },
        Action {
            id: "dashboard_e2e".into(),
            title: "Dashboard E2E".into(),
            desc: "Run browser end-to-end tests".into(),
            cmd: "dashboard-ui-e2e".into(),
            args: vec![],
            risk: "MED".into(),
            category: "test".into(),
            impact: "Executes browser E2E automation.".into(),
        },
        Action {
            id: "quality_gate".into(),
            title: "Tooling Quality Gate".into(),
            desc: "Run full gate checks".into(),
            cmd: "tooling-quality-gate".into(),
            args: vec![],
            risk: "HIGH".into(),
            category: "gate".into(),
            impact: "Runs full quality gate and acceptance checks.".into(),
        },
        Action {
            id: "open_report".into(),
            title: "Open Report".into(),
            desc: "Open modern dashboard report".into(),
            cmd: "open-report".into(),
            args: vec!["-ReportTarget".into(), "ui".into()],
            risk: "INFO".into(),
            category: "dashboard".into(),
            impact: "Opens local report UI in browser.".into(),
        },
        Action {
            id: "crash_diagnostics".into(),
            title: "Crash Diagnostics Bundle".into(),
            desc: "Collect diagnostics artifact bundle".into(),
            cmd: "collect-diagnostics".into(),
            args: vec![],
            risk: "MED".into(),
            category: "recovery".into(),
            impact: "Collects diagnostics zip and metadata for triage.".into(),
        },
        Action {
            id: "crash_triage".into(),
            title: "Crash Triage".into(),
            desc: "Generate triage report from recent failures".into(),
            cmd: "triage".into(),
            args: vec![],
            risk: "MED".into(),
            category: "recovery".into(),
            impact: "Builds actionable failure triage report.".into(),
        },
    ]
}

// ── Job ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Done => "done",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub action: String,
    pub priority: String,
    pub status: JobStatus,
    pub source: String,
    pub queued_utc: DateTime<Utc>,
    pub started_utc: Option<DateTime<Utc>>,
    pub finished_utc: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub output: Vec<String>,
    pub error: Option<String>,
}

impl Job {
    pub fn new(
        action: impl Into<String>,
        priority: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Job {
            id: Uuid::new_v4().to_string(),
            action: action.into(),
            priority: priority.into(),
            status: JobStatus::Queued,
            source: source.into(),
            queued_utc: Utc::now(),
            started_utc: None,
            finished_utc: None,
            exit_code: None,
            output: vec![],
            error: None,
        }
    }

    pub fn summary_value(&self) -> Value {
        json!({
            "id": self.id,
            "action": self.action,
            "priority": self.priority,
            "status": self.status.as_str(),
            "source": self.source,
            "queued_utc": self.queued_utc.to_rfc3339(),
            "started_utc": self.started_utc.map(|d| d.to_rfc3339()),
            "finished_utc": self.finished_utc.map(|d| d.to_rfc3339()),
            "exit_code": self.exit_code,
        })
    }

    pub fn detail_value(&self) -> Value {
        json!({
            "id": self.id,
            "action": self.action,
            "priority": self.priority,
            "status": self.status.as_str(),
            "source": self.source,
            "queued_utc": self.queued_utc.to_rfc3339(),
            "started_utc": self.started_utc.map(|d| d.to_rfc3339()),
            "finished_utc": self.finished_utc.map(|d| d.to_rfc3339()),
            "exit_code": self.exit_code,
            "output": self.output,
            "error": self.error,
        })
    }
}

// ── Scheduler ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub action: String,
    pub interval_sec: u64,
    pub priority: String,
    pub enabled: bool,
    pub last_run_utc: Option<DateTime<Utc>>,
    pub next_run_utc: DateTime<Utc>,
    pub source: String,
}

// ── Role policy ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolePolicy {
    pub max_risk: String,
    pub denied_actions: Vec<String>,
    pub denied_categories: Vec<String>,
}

impl RolePolicy {
    pub fn default_for(role: &str) -> Self {
        match role {
            "viewer" => RolePolicy {
                max_risk: "INFO".into(),
                denied_actions: vec!["*".into()],
                denied_categories: vec![],
            },
            "operator" => RolePolicy {
                max_risk: "HIGH".into(),
                denied_actions: vec!["doctor_fix".into(), "quality_gate".into()],
                denied_categories: vec![],
            },
            // admin
            _ => RolePolicy {
                max_risk: "HIGH".into(),
                denied_actions: vec![],
                denied_categories: vec![],
            },
        }
    }
}

pub fn default_role_policies() -> HashMap<String, RolePolicy> {
    let mut m = HashMap::new();
    m.insert("viewer".into(), RolePolicy::default_for("viewer"));
    m.insert("operator".into(), RolePolicy::default_for("operator"));
    m.insert("admin".into(), RolePolicy::default_for("admin"));
    m
}

// ── Host ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub role_hint: String,
    pub token: String,
    pub last_seen_utc: Option<DateTime<Utc>>,
    pub reachable: Option<bool>,
    pub capabilities: Vec<String>,
}

// ── Policy trace entry ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyTrace {
    pub ts_utc: DateTime<Utc>,
    pub source: String,
    pub role: String,
    pub action: String,
    pub category: String,
    pub risk: String,
    pub allowed: bool,
    pub reason: String,
}

// ── Confirmation ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confirmation {
    pub id: String,
    pub action: String,
    pub priority: String,
    pub issued_utc: DateTime<Utc>,
    pub expires_utc: DateTime<Utc>,
    pub role: String,
}

// ── Recent activity ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    pub id: String,
    pub action: String,
    pub status: String,
    pub started_utc: Option<DateTime<Utc>>,
    pub finished_utc: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub kind: String,
    pub ts_utc: DateTime<Utc>,
    pub related_id: Option<String>,
    pub action: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub detail: serde_json::Value,
}

impl AgentEvent {
    pub fn matches_filters(&self, kind_filter: Option<&str>, action_filter: Option<&str>) -> bool {
        let kind_ok = kind_filter
            .map(|kind| self.kind.to_lowercase() == kind)
            .unwrap_or(true);
        let action_ok = action_filter
            .map(|needle| {
                self.action
                    .as_ref()
                    .map(|action| action.to_lowercase().contains(needle))
                    .unwrap_or(false)
            })
            .unwrap_or(true);
        kind_ok && action_ok
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    pub key: String,
    pub created_utc: DateTime<Utc>,
    pub route: String,
    pub action: String,
    pub job_ids: Vec<String>,
}
