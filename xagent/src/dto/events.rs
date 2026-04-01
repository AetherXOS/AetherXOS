use std::collections::BTreeMap;

use serde::Serialize;

use crate::models::AgentEvent;

#[derive(Debug, Clone, Serialize)]
pub struct EventDto {
    pub id: String,
    pub kind: String,
    pub ts_utc: String,
    pub related_id: Option<String>,
    pub action: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub detail: serde_json::Value,
}

impl From<&AgentEvent> for EventDto {
    fn from(value: &AgentEvent) -> Self {
        Self {
            id: value.id.clone(),
            kind: value.kind.clone(),
            ts_utc: value.ts_utc.to_rfc3339(),
            related_id: value.related_id.clone(),
            action: value.action.clone(),
            status: value.status.clone(),
            source: value.source.clone(),
            detail: value.detail.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EventsListDto {
    pub events: Vec<EventDto>,
    pub returned: usize,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventActionCountDto {
    pub action: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventsStatsDto {
    pub total: usize,
    pub kind_filter: Option<String>,
    pub since_minutes: Option<i64>,
    pub from_ts: Option<String>,
    pub kinds: BTreeMap<String, usize>,
    pub actions_top: Vec<EventActionCountDto>,
}
