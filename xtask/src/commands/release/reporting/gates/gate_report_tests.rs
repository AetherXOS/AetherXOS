use super::*;

#[test]
fn render_gate_report_md_shows_regressions_and_improvements() {
    let doc = GateReportDoc {
        generated_utc: "2026-04-23T00:00:00Z".to_string(),
        strict: true,
        baseline_path: "reports/tooling/ci_bundle_prev.json".to_string(),
        baseline_created: false,
        current_overall_ok: false,
        regressions: vec!["abi_drift".to_string(), "linux_abi_semantic_matrix".to_string()],
        improvements: vec!["qemu_boot_markers".to_string()],
    };

    let md = render_gate_report_md(&doc);

    assert!(md.contains("# CI Gate Report"));
    assert!(md.contains("baseline_path: reports/tooling/ci_bundle_prev.json"));
    assert!(md.contains("regressions: 2"));
    assert!(md.contains("improvements: 1"));
    assert!(md.contains("## Regressions"));
    assert!(md.contains("abi_drift"));
    assert!(md.contains("## Improvements"));
    assert!(md.contains("qemu_boot_markers"));
}
