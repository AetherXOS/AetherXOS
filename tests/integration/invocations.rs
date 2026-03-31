use crate::common::{ctx, plan, tool};

#[test]
fn integration_plan_runs_only_mandatory_steps_without_optional_gates() {
    let actual = tool::expected_invocations(
        &plan::integration(),
        ctx::root(),
        tool::FAKE_HOST_TRIPLE,
        &[],
        &[],
        &[],
    );

    let expected = vec![
        nextest(),
        clippy(),
        cargo_test("host_rust_tests/Cargo.toml"),
        cargo_test("host_tools/scheduler_host_tests/Cargo.toml"),
        cargo_test("agent/Cargo.toml"),
        probe("fuzz"),
    ];

    assert_eq!(actual, expected);
}

#[test]
fn integration_plan_expands_all_optional_steps_when_gates_and_tools_exist() {
    let actual = tool::expected_invocations(
        &plan::integration(),
        ctx::root(),
        tool::FAKE_HOST_TRIPLE,
        &[
            "HYPERCORE_RUN_KASAN",
            "HYPERCORE_RUN_KMSAN",
            "HYPERCORE_RUN_UBSAN",
            "HYPERCORE_RUN_VIRTME",
            "HYPERCORE_RUN_FUZZ",
        ],
        &["fuzz"],
        &["vng"],
    );

    let expected = vec![
        nextest(),
        clippy(),
        cargo_test("host_rust_tests/Cargo.toml"),
        cargo_test("host_tools/scheduler_host_tests/Cargo.toml"),
        cargo_test("agent/Cargo.toml"),
        step(&["vng", "--version"]),
        probe("fuzz"),
        step(&[
            "cargo",
            "fuzz",
            "build",
            "kernel_config_bytes",
            "--manifest-path",
            "Cargo.toml",
        ]),
    ];

    assert_eq!(actual, expected);
}

fn nextest() -> Vec<String> {
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
        "integration_tests".into(),
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

fn cargo_test(manifest_path: &str) -> Vec<String> {
    vec![
        "cargo".into(),
        "test".into(),
        "--manifest-path".into(),
        repo(manifest_path),
        "--tests".into(),
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
