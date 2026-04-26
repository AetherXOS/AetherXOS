use super::*;

#[test]
fn render_support_diagnostics_md_lists_status_and_commands() {
    let doc = SupportDiagnosticsDoc {
        generated_utc: "2026-04-23T00:00:00Z".to_string(),
        strict: true,
        overall_ok: false,
        commands: vec![
            "cargo run -p xtask -- release doctor --strict".to_string(),
            "cargo run -p xtask -- release ci-bundle --strict".to_string(),
        ],
        status: vec![
            SupportCheck {
                id: "doctor".to_string(),
                ok: true,
                detail: "overall_ok=true".to_string(),
            },
            SupportCheck {
                id: "ci_bundle".to_string(),
                ok: false,
                detail: "overall_ok=false".to_string(),
            },
        ],
    };

    let md = render_support_diagnostics_md(&doc);

    assert!(md.contains("# Support Diagnostics"));
    assert!(md.contains("overall_ok: false"));
    assert!(md.contains("## Status"));
    assert!(md.contains("[x] doctor"));
    assert!(md.contains("[ ] ci_bundle"));
    assert!(md.contains("## Suggested Commands"));
    assert!(md.contains("release ci-bundle --strict"));
}
