use super::*;

#[test]
fn select_scoring_checks_prefers_perf_relevant_subset() {
    let checks = vec![
        serde_json::json!({"id": "host_smoke", "ok": true}),
        serde_json::json!({"id": "abi_drift", "ok": false}),
        serde_json::json!({"id": "linux_abi_semantic_matrix", "ok": true}),
    ];

    let selected = select_scoring_checks(&checks);

    assert_eq!(selected.len(), 2);
    assert!(selected.iter().all(|check| {
        matches!(
            check.get("id").and_then(|v| v.as_str()),
            Some("abi_drift" | "linux_abi_semantic_matrix")
        )
    }));
}

#[test]
fn select_scoring_checks_falls_back_to_all_checks_when_subset_is_empty() {
    let checks = vec![
        serde_json::json!({"id": "host_smoke", "ok": true}),
        serde_json::json!({"id": "integration", "ok": false}),
    ];

    let selected = select_scoring_checks(&checks);

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0]["id"], "host_smoke");
    assert_eq!(selected[1]["id"], "integration");
}

#[test]
fn completion_pct_and_failed_count_track_state_consistently() {
    let checks = vec![
        serde_json::json!({"id": "abi_drift", "ok": true}),
        serde_json::json!({"id": "linux_abi_semantic_matrix", "ok": false}),
        serde_json::json!({"id": "linux_abi_workload_catalog", "ok": true}),
    ];

    let failed = failed_check_count(&checks);
    let pct = completion_pct(&checks, failed);

    assert_eq!(failed, 1);
    assert!((pct - 66.7).abs() < 0.1);
}
