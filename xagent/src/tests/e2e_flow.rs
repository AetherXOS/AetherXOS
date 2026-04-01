use rocket::http::ContentType;

use super::support::{admin_header, test_client};

#[test]
fn end_to_end_run_to_stream_contract() {
    let client = test_client();
    let token = admin_header();

    let run = client
        .post("/run")
        .header(token.clone())
        .header(ContentType::JSON)
        .body(r#"{"action":"doctor","priority":"normal"}"#)
        .dispatch();
    let run_json: serde_json::Value = run.into_json().expect("run json");
    assert_eq!(run_json["ok"], true);
    let job_id = run_json["id"].as_str().expect("job id").to_string();

    let job = client
        .get(format!("/job?id={}", job_id))
        .header(token.clone())
        .dispatch();
    let job_json: serde_json::Value = job.into_json().expect("job json");
    assert_eq!(job_json["ok"], true);
    assert_eq!(job_json["job"]["id"], job_id);

    let job_events = client
        .get(format!("/job/events?id={}&follow=false", job_id))
        .header(token.clone())
        .dispatch();
    let sse = job_events.into_string().expect("job events sse");
    assert!(sse.contains("snapshot") || sse.contains("complete") || sse.contains("heartbeat"));

    let global_events = client
        .get("/events?action=doctor&limit=20")
        .header(token)
        .dispatch();
    let global_events_json: serde_json::Value = global_events.into_json().expect("global events json");
    assert_eq!(global_events_json["ok"], true);
    assert!(global_events_json["returned"].as_u64().unwrap_or(0) >= 1);
}
