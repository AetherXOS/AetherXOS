use crate::dto::events::EventDto;
use crate::models::AgentEvent;

#[test]
fn agent_event_filtering_is_case_insensitive() {
    let event = AgentEvent {
        id: "u1".into(),
        kind: "Job".into(),
        ts_utc: chrono::Utc::now(),
        related_id: None,
        action: Some("DoCtOr".into()),
        status: Some("done".into()),
        source: Some("unit".into()),
        detail: serde_json::json!({"k": 1}),
    };

    assert!(event.matches_filters(Some("job"), Some("doctor")));
    assert!(event.matches_filters(Some("job"), None));
    assert!(!event.matches_filters(Some("host"), None));
    assert!(!event.matches_filters(Some("job"), Some("qemu")));
}

#[test]
fn event_dto_conversion_preserves_core_fields() {
    let event = AgentEvent {
        id: "u2".into(),
        kind: "job".into(),
        ts_utc: chrono::Utc::now(),
        related_id: Some("job-42".into()),
        action: Some("doctor".into()),
        status: Some("running".into()),
        source: Some("unit".into()),
        detail: serde_json::json!({"hello": "world"}),
    };

    let dto = EventDto::from(&event);
    assert_eq!(dto.id, "u2");
    assert_eq!(dto.kind, "job");
    assert_eq!(dto.related_id.as_deref(), Some("job-42"));
    assert_eq!(dto.action.as_deref(), Some("doctor"));
    assert_eq!(dto.status.as_deref(), Some("running"));
    assert_eq!(dto.source.as_deref(), Some("unit"));
    assert_eq!(dto.detail["hello"], "world");
}
