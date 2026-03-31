use crate::common::{ctx, plan, tool};

#[test]
fn fast_plan_executes_full_host_flow_when_optional_tools_are_enabled() {
    let actual = tool::expected_invocations(
        &plan::fast(),
        ctx::root(),
        tool::FAKE_HOST_TRIPLE,
        &[
            "HYPERCORE_ENABLE_GEIGER",
            "HYPERCORE_ENABLE_RUDRA",
            "HYPERCORE_ENABLE_AUDIT",
        ],
        &["geiger", "rudra", "audit"],
        &[],
    );

    let expected = vec![
        nextest("fast"),
        clippy(),
        rustfmt(),
        probe("geiger"),
        geiger(),
        probe("rudra"),
        rudra(),
        probe("audit"),
        step(&["cargo", "audit"]),
    ];

    assert_eq!(actual, expected);
}

#[test]
fn fast_plan_keeps_optional_tools_probe_only_when_gate_or_tool_is_missing() {
    let actual = tool::expected_invocations(
        &plan::fast(),
        ctx::root(),
        tool::FAKE_HOST_TRIPLE,
        &["HYPERCORE_ENABLE_GEIGER"],
        &["geiger"],
        &[],
    );

    let expected = vec![
        nextest("fast"),
        clippy(),
        rustfmt(),
        probe("geiger"),
        geiger(),
        probe("rudra"),
        probe("audit"),
    ];

    assert_eq!(actual, expected);
}

fn nextest(test_name: &str) -> Vec<String> {
    vec![
        "cargo".into(),
        "nextest".into(),
        "run".into(),
        "--config-file".into(),
        repo(".config/nextest.toml"),
        "--target".into(),
        host(),
        "--features".into(),
        features(),
        "--test".into(),
        test_name.into(),
    ]
}

fn clippy() -> Vec<String> {
    vec![
        "cargo".into(),
        "clippy".into(),
        "--manifest-path".into(),
        "Cargo.toml".into(),
        "--lib".into(),
        "--target".into(),
        host(),
        "--features".into(),
        features(),
        "--".into(),
        "-A".into(),
        "warnings".into(),
        "-A".into(),
        "unused".into(),
        "-A".into(),
        "dead_code".into(),
        "-A".into(),
        "unused_imports".into(),
        "-A".into(),
        "unused_variables".into(),
        "-A".into(),
        "unused_mut".into(),
        "-A".into(),
        "unsafe_op_in_unsafe_fn".into(),
        "-A".into(),
        "clippy::all".into(),
    ]
}

fn rustfmt() -> Vec<String> {
    step(&[
        "cargo",
        "fmt",
        "--manifest-path",
        "Cargo.toml",
        "--all",
        "--check",
    ])
}

fn geiger() -> Vec<String> {
    vec![
        "cargo".into(),
        "geiger".into(),
        "--manifest-path".into(),
        "Cargo.toml".into(),
        "--all-targets".into(),
        "--target".into(),
        host(),
        "--features".into(),
        features(),
    ]
}

fn rudra() -> Vec<String> {
    vec![
        "cargo".into(),
        "rudra".into(),
        "--manifest-path".into(),
        "Cargo.toml".into(),
        "--all-targets".into(),
        "--target".into(),
        host(),
        "--features".into(),
        features(),
    ]
}

fn probe(subcommand: &str) -> Vec<String> {
    vec!["cargo".into(), subcommand.into(), "--help".into()]
}

fn step(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_string()).collect()
}

fn repo(path: &str) -> String {
    ctx::path(path).display().to_string()
}

fn host() -> String {
    tool::FAKE_HOST_TRIPLE.to_string()
}

fn features() -> String {
    "kernel_test_mode,vfs,drivers".to_string()
}
