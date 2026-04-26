use super::*;

#[test]
fn render_release_notes_md_includes_gate_summary_and_remediation_plan() {
    let checks = vec![
        serde_json::json!({"id": "doctor", "ok": true}),
        serde_json::json!({"id": "ci_bundle", "ok": false}),
    ];
    let plan = vec![
        serde_json::json!("1) Run xtask gate-fixup"),
        serde_json::json!("- [ci_bundle] rerun release ci-bundle --strict"),
    ];

    let notes = render_release_notes_md(&checks, &plan, false);

    assert!(notes.contains("# Release Notes (Auto)"));
    assert!(notes.contains("overall_ok: false"));
    assert!(notes.contains("## Gate Summary"));
    assert!(notes.contains("[x] doctor"));
    assert!(notes.contains("[ ] ci_bundle"));
    assert!(notes.contains("## Remediation Plan"));
    assert!(notes.contains("Run xtask gate-fixup"));
    assert!(notes.contains("rerun release ci-bundle --strict"));
}
