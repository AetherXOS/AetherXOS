use crate::common::{ctx, plan, tool};

#[test]
fn nightly_plan_runs_mandatory_steps_and_probes_optional_subcommands() {
    let actual = tool::expected_invocations(
        &plan::nightly(),
        ctx::root(),
        tool::FAKE_HOST_TRIPLE,
        &[],
        &[],
        &[],
    );

    let expected = vec![nextest(), clippy(), probe("kani"), probe("flamegraph")];

    assert_eq!(actual, expected);
}

#[test]
fn nightly_plan_expands_formal_and_perf_steps_when_tools_are_available() {
    let actual = tool::expected_invocations(
        &plan::nightly(),
        ctx::root(),
        tool::FAKE_HOST_TRIPLE,
        &[
            "HYPERCORE_RUN_SYZKALLER",
            "HYPERCORE_RUN_TLAPLUS",
            "HYPERCORE_RUN_KANI",
            "HYPERCORE_RUN_ISABELLE",
            "HYPERCORE_RUN_FLAMEGRAPH",
        ],
        &["kani", "flamegraph"],
        &["syz-manager", "tlc", "isabelle"],
    );

    let expected = vec![
        nextest(),
        clippy(),
        step(&[
            "syz-manager",
            "-config",
            &repo("formal/syzkaller/hypercore.cfg"),
        ]),
        step(&["tlc", &repo("formal/tla/KernelConfigOverrides.tla")]),
        probe("kani"),
        step(&[
            "cargo",
            "kani",
            "--manifest-path",
            &repo("formal/kani/Cargo.toml"),
        ]),
        step(&["isabelle", "build", "-D", &repo("formal/isabelle")]),
        probe("flamegraph"),
        step(&[
            "cargo",
            "flamegraph",
            "--manifest-path",
            &repo("host_tools/scheduler_host_tests/Cargo.toml"),
            "--test",
            "scheduler_runtime",
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
        "nightly".into(),
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
