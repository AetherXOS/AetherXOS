use rocket::http::{ContentType, Status};

use crate::state::AppState;

use super::support::{admin_header, test_client};

#[test]
fn jobs_filter_and_validation_contract() {
    let client = test_client();
    let token = admin_header();

    {
        let state = client.rocket().state::<AppState>().expect("state");
        let mut inner = state.write();
        inner.jobs.insert(
            "job-1".into(),
            crate::models::Job {
                id: "job-1".into(),
                action: "doctor".into(),
                priority: "normal".into(),
                status: crate::models::JobStatus::Done,
                source: "api:test".into(),
                queued_utc: chrono::Utc::now(),
                started_utc: None,
                finished_utc: None,
                exit_code: Some(0),
                output: vec!["ok".into()],
                error: None,
            },
        );
    }

    let jobs = client
        .get("/jobs?status=done&limit=10")
        .header(token.clone())
        .dispatch();
    assert_eq!(jobs.status(), Status::Ok);
    let jobs_json: serde_json::Value = jobs.into_json().expect("jobs json");
    assert_eq!(jobs_json["total"], 1);
    assert_eq!(jobs_json["jobs"][0]["id"], "job-1");

    let invalid = client
        .post("/run")
        .header(token)
        .header(ContentType::JSON)
        .body(r#"{"action":"","priority":"normal"}"#)
        .dispatch();
    assert_eq!(invalid.status(), Status::Ok);
    let invalid_json: serde_json::Value = invalid.into_json().expect("invalid json");
    assert_eq!(invalid_json["ok"], false);
    assert_eq!(invalid_json["code"], "invalid_action");
}

#[test]
fn idempotency_and_retry_contract() {
    let client = test_client();
    let token = admin_header();

    let first = client
        .post("/run")
        .header(token.clone())
        .header(rocket::http::Header::new("X-Idempotency-Key", "same-key-1"))
        .header(ContentType::JSON)
        .body(r#"{"action":"doctor","priority":"normal"}"#)
        .dispatch();
    let first_json: serde_json::Value = first.into_json().expect("first json");
    assert_eq!(first_json["ok"], true);
    assert_eq!(first_json["replayed"], false);
    let job_id = first_json["id"].as_str().expect("job id").to_string();

    let second = client
        .post("/run")
        .header(token.clone())
        .header(rocket::http::Header::new("X-Idempotency-Key", "same-key-1"))
        .header(ContentType::JSON)
        .body(r#"{"action":"doctor","priority":"normal"}"#)
        .dispatch();
    let second_json: serde_json::Value = second.into_json().expect("second json");
    assert_eq!(second_json["ok"], true);
    assert_eq!(second_json["replayed"], true);
    assert_eq!(second_json["id"], job_id);

    let retry = client
        .post("/job/retry")
        .header(token)
        .header(ContentType::JSON)
        .body(format!(r#"{{"id":"{}"}}"#, job_id))
        .dispatch();
    let retry_json: serde_json::Value = retry.into_json().expect("retry json");
    assert_eq!(retry_json["ok"], true);
    assert_ne!(retry_json["id"], job_id);
}

#[test]
fn dispatch_job_events_sse_contract() {
    let client = test_client();
    let token = admin_header();

    {
        let state = client.rocket().state::<AppState>().expect("state");
        let mut inner = state.write();
        inner.jobs.insert(
            "dispatch-job-1".into(),
            crate::models::Job {
                id: "dispatch-job-1".into(),
                action: "doctor".into(),
                priority: "normal".into(),
                status: crate::models::JobStatus::Done,
                source: "dispatch:host:local".into(),
                queued_utc: chrono::Utc::now(),
                started_utc: Some(chrono::Utc::now()),
                finished_utc: Some(chrono::Utc::now()),
                exit_code: Some(0),
                output: vec!["line-a".into(), "line-b".into()],
                error: None,
            },
        );
    }

    let stream = client
        .get("/dispatch/job/events?id=dispatch-job-1&follow=false")
        .header(token)
        .dispatch();
    assert_eq!(stream.status(), Status::Ok);
    let body = stream.into_string().expect("dispatch events sse");
    assert!(body.contains("\"type\":\"snapshot\""));
    assert!(body.contains("\"type\":\"line\""));
    assert!(body.contains("\"type\":\"complete\""));
}
