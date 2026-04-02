use rocket::http::Status;

use super::support::{admin_header, test_client};

#[test]
fn plugin_registry_contract() {
    let client = test_client();
    let token = admin_header();

    let plugins = client.get("/plugins").header(token.clone()).dispatch();
    assert_eq!(plugins.status(), Status::Ok);
    let plugins_json: serde_json::Value = plugins.into_json().expect("plugins json");
    assert_eq!(plugins_json["ok"], true);
    assert!(plugins_json["plugins"].is_array());

    let list = plugins_json["plugins"].as_array().expect("plugins array");
    if let Some(first) = list.first() {
        let name = first["name"].as_str().expect("plugin name");
        let detail = client
            .get(format!("/plugins/{}", name))
            .header(token)
            .dispatch();
        assert_eq!(detail.status(), Status::Ok);
        let detail_json: serde_json::Value = detail.into_json().expect("detail json");
        assert_eq!(detail_json["ok"], true);
        assert_eq!(detail_json["plugin"]["name"], name);
    }
}
