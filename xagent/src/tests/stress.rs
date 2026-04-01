use rocket::http::Status;

use crate::models::AgentEvent;
use crate::state::AppState;

use super::support::{admin_header, test_client};

#[test]
fn stress_events_stream_large_replay_is_bounded() {
    let client = test_client();
    let token = admin_header();
    let state = client.rocket().state::<AppState>().expect("state");

    {
        let mut inner = state.write();
        for i in 0..1200 {
            inner.events.push(AgentEvent {
                id: format!("stress-{}", i),
                kind: if i % 2 == 0 { "job".into() } else { "host".into() },
                ts_utc: chrono::Utc::now() - chrono::Duration::seconds((1200 - i) as i64),
                related_id: None,
                action: Some("stress".into()),
                status: Some("ok".into()),
                source: Some("stress-test".into()),
                detail: serde_json::json!({"i": i}),
            });
        }
    }

    let stream = client
        .get("/events/stream?follow=false&replay_limit=50")
        .header(token.clone())
        .dispatch();
    assert_eq!(stream.status(), Status::Ok);
    let body = stream.into_string().expect("stream body");
    let data_count = body.matches("data:").count();
    assert!(data_count <= 50, "stream replay should be bounded, got {data_count}");

    let stats = client
        .get("/events/stats?since_minutes=120")
        .header(token)
        .dispatch();
    assert_eq!(stats.status(), Status::Ok);
    let stats_json: serde_json::Value = stats.into_json().expect("stats json");
    assert!(stats_json["total"].as_u64().unwrap_or(0) >= 1000);
}
