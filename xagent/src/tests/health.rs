use rocket::http::Status;

use super::support::{admin_header, test_client};

#[test]
fn health_and_ready_contract() {
    let client = test_client();
    let token = admin_header();

    let health = client.get("/health").header(token.clone()).dispatch();
    assert_eq!(health.status(), Status::Ok);
    let health_json: serde_json::Value = health.into_json().expect("health json");
    assert_eq!(health_json["ok"], true);
    assert_eq!(health_json["role"], "admin");

    let ready = client.get("/ready").header(token).dispatch();
    assert_eq!(ready.status(), Status::Ok);
    let ready_json: serde_json::Value = ready.into_json().expect("ready json");
    assert_eq!(ready_json["ok"], true);
    assert!(ready_json["ready"].is_boolean());
}

#[test]
fn removed_openapi_contract() {
    let client = test_client();
    let spec = client.get("/openapi.json").dispatch();
    assert_eq!(spec.status(), Status::NotFound);
}
