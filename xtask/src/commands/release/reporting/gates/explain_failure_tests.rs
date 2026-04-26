use super::*;

#[test]
fn render_explain_failure_md_includes_action_plan() {
    let doc = ExplainFailureDoc {
        generated_utc: "2026-04-23T00:00:00Z".to_string(),
        strict: false,
        overall_ok: false,
        issue_count: 2,
        action_plan: vec![
            "1) Run xtask gate-fixup".to_string(),
            "- [scorecard_gate_failed] rerun release p0-p1-nightly".to_string(),
        ],
    };

    let md = render_explain_failure_md(&doc);

    assert!(md.contains("# Explain Failure"));
    assert!(md.contains("issue_count: 2"));
    assert!(md.contains("## Action Plan"));
    assert!(md.contains("Run xtask gate-fixup"));
    assert!(md.contains("scorecard_gate_failed"));
}
