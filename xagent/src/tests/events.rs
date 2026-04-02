use rocket::http::{ContentType, Status};

use crate::models::AgentEvent;
use crate::state::AppState;

use super::support::{admin_header, test_client};

fn seed_event(state: &AppState, id: &str, kind: &str, action: Option<&str>, minutes_ago: i64) {
    let mut inner = state.write();
    inner.events.push(AgentEvent {
        id: id.into(),
        kind: kind.into(),
        ts_utc: chrono::Utc::now() - chrono::Duration::minutes(minutes_ago),
        related_id: None,
        action: action.map(|v| v.to_string()),
        status: Some("ok".into()),
        source: Some("test".into()),
        detail: serde_json::json!({"seed": true}),
    });
}

#[test]
fn prune_events_and_host_heartbeat_contract() {
    let client = test_client();
    let token = admin_header();

    {
        let state = client.rocket().state::<AppState>().expect("state");
        let mut inner = state.write();
        inner.jobs.insert(
            "old-job".into(),
            crate::models::Job {
                id: "old-job".into(),
                action: "doctor".into(),
                priority: "low".into(),
                status: crate::models::JobStatus::Done,
                source: "api:test".into(),
                queued_utc: chrono::Utc::now() - chrono::Duration::hours(48),
                started_utc: None,
                finished_utc: Some(chrono::Utc::now() - chrono::Duration::hours(36)),
                exit_code: Some(0),
                output: vec![],
                error: None,
            },
        );
    }

    let heartbeat = client
        .post("/hosts/heartbeat")
        .header(token.clone())
        .header(ContentType::JSON)
        .body(r#"{"id":"local","reachable":true,"capabilities":["admin","metrics"]}"#)
        .dispatch();
    let heartbeat_json: serde_json::Value = heartbeat.into_json().expect("heartbeat json");
    assert_eq!(heartbeat_json["ok"], true);
    assert_eq!(heartbeat_json["host"]["reachable"], true);

    let events = client
        .get("/events?limit=20")
        .header(token.clone())
        .dispatch();
    let events_json: serde_json::Value = events.into_json().expect("events json");
    assert_eq!(events_json["ok"], true);
    assert!(events_json["events"].is_array());

    let stream = client
        .get("/events/stream?follow=false")
        .header(token.clone())
        .dispatch();
    assert_eq!(stream.status(), Status::Ok);
    let sse_body = stream.into_string().expect("events sse");
    assert!(sse_body.contains("data:") || sse_body.contains("heartbeat"));

    let prune = client
        .delete("/jobs/prune?hours=24")
        .header(token)
        .dispatch();
    let prune_json: serde_json::Value = prune.into_json().expect("prune json");
    assert_eq!(prune_json["ok"], true);
    assert_eq!(prune_json["removed"], 1);
}

#[test]
fn events_filter_and_stats_contract() {
    let client = test_client();
    let token = admin_header();
    let state = client.rocket().state::<AppState>().expect("state");

    seed_event(state, "e1", "job", Some("doctor"), 10);
    seed_event(state, "e2", "host", Some("heartbeat"), 5);
    seed_event(state, "e3", "job", Some("doctor"), 2);

    let filtered = client
        .get("/events?kind=job&action=doct&limit=10")
        .header(token.clone())
        .dispatch();
    assert_eq!(filtered.status(), Status::Ok);
    let filtered_json: serde_json::Value = filtered.into_json().expect("filtered json");
    assert_eq!(filtered_json["returned"], 2);

    let stats = client
        .get("/events/stats?since_minutes=60")
        .header(token)
        .dispatch();
    assert_eq!(stats.status(), Status::Ok);
    let stats_json: serde_json::Value = stats.into_json().expect("stats json");
    assert!(stats_json["total"].as_u64().unwrap_or(0) >= 3);
    assert!(stats_json["kinds"]["job"].as_u64().unwrap_or(0) >= 2);

    let from_ts = (chrono::Utc::now() - chrono::Duration::minutes(6))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let filtered_time = client
        .get(format!("/events?kind=job&from_ts={}", from_ts))
        .header(admin_header())
        .dispatch();
    assert_eq!(filtered_time.status(), Status::Ok);
    let filtered_time_json: serde_json::Value =
        filtered_time.into_json().expect("time filtered json");
    assert_eq!(filtered_time_json["returned"], 1);

    let invalid_ts = client
        .get("/events?from_ts=not-a-date")
        .header(admin_header())
        .dispatch();
    assert_eq!(invalid_ts.status(), Status::Ok);
    let invalid_ts_json: serde_json::Value = invalid_ts.into_json().expect("invalid ts json");
    assert_eq!(invalid_ts_json["ok"], false);
    assert_eq!(invalid_ts_json["code"], "invalid_from_ts");
}

#[test]
fn events_stream_replay_limit_contract() {
    let client = test_client();
    let token = admin_header();
    let state = client.rocket().state::<AppState>().expect("state");

    seed_event(state, "r1", "job", Some("a"), 3);
    seed_event(state, "r2", "job", Some("b"), 2);
    seed_event(state, "r3", "job", Some("c"), 1);

    let stream = client
        .get("/events/stream?follow=false&replay_limit=1")
        .header(token)
        .dispatch();
    assert_eq!(stream.status(), Status::Ok);
    let body = stream.into_string().expect("stream body");
    let data_count = body.matches("data:").count();
    assert!(
        data_count <= 1,
        "expected <=1 data frames, got body: {body}"
    );
}

#[test]
fn events_stream_from_ts_contract() {
    let client = test_client();
    let token = admin_header();
    let state = client.rocket().state::<AppState>().expect("state");

    seed_event(state, "t1", "job", Some("old"), 120);
    seed_event(state, "t2", "job", Some("new"), 1);

    let from_ts = (chrono::Utc::now() - chrono::Duration::minutes(10))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let stream = client
        .get(format!("/events/stream?follow=false&from_ts={}", from_ts))
        .header(token)
        .dispatch();
    assert_eq!(stream.status(), Status::Ok);
    let body = stream.into_string().expect("stream body");
    assert!(body.contains("\"id\":\"t2\""));
    assert!(!body.contains("\"id\":\"t1\""));
}

#[test]
fn events_cursor_pagination_contract() {
    let client = test_client();
    let token = admin_header();
    let state = client.rocket().state::<AppState>().expect("state");

    seed_event(state, "c1", "job", Some("a"), 5);
    seed_event(state, "c2", "job", Some("b"), 4);
    seed_event(state, "c3", "job", Some("c"), 3);

    let page1 = client
        .get("/events?kind=job&limit=2")
        .header(token.clone())
        .dispatch();
    assert_eq!(page1.status(), Status::Ok);
    let page1_json: serde_json::Value = page1.into_json().expect("page1 json");
    assert_eq!(page1_json["returned"], 2);
    assert!(page1_json["next_cursor"].is_string());

    let cursor = page1_json["next_cursor"].as_str().expect("cursor");
    let page2 = client
        .get(format!("/events?kind=job&limit=2&cursor={}", cursor))
        .header(token.clone())
        .dispatch();
    assert_eq!(page2.status(), Status::Ok);
    let page2_json: serde_json::Value = page2.into_json().expect("page2 json");
    assert_eq!(page2_json["returned"], 1);
    assert!(page2_json["next_cursor"].is_null());

    let invalid_cursor = client
        .get("/events?kind=job&limit=2&cursor=does-not-exist")
        .header(token)
        .dispatch();
    assert_eq!(invalid_cursor.status(), Status::Ok);
    let invalid_cursor_json: serde_json::Value =
        invalid_cursor.into_json().expect("invalid cursor json");
    assert_eq!(invalid_cursor_json["ok"], false);
    assert_eq!(invalid_cursor_json["code"], "invalid_cursor");
}
