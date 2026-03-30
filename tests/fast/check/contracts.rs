use std::collections::HashSet;

use crate::common::tool::{
    expected_invocations, run_script, Availability, TierPlan, FAKE_HOST_TRIPLE,
};
use crate::common::{ctx, fs, plan};
use toml::Value;

#[test]
fn tier_plans_stay_well_formed_and_asset_backed() {
    assert_tier_contract(
        &plan::fast(),
        &["nextest", "clippy", "rustfmt", "geiger", "rudra", "audit"],
    );
    assert_tier_contract(
        &plan::integration(),
        &["nextest", "kasan", "kmsan", "ubsan", "virtme", "cargofuzz"],
    );
    assert_tier_contract(
        &plan::nightly(),
        &[
            "nextest",
            "syzkaller",
            "tlaplus",
            "kani",
            "isabelle",
            "flamegraph",
        ],
    );
}

#[test]
fn cargo_test_targets_and_nextest_profiles_stay_aligned() {
    let cargo: Value = toml::from_str(&ctx::read("Cargo.toml")).expect("Cargo.toml should parse");
    let tests = cargo["test"]
        .as_array()
        .expect("Cargo.toml should define test targets");

    let entries: Vec<_> = tests
        .iter()
        .map(|entry| {
            (
                entry["name"].as_str().expect("test name"),
                entry["path"].as_str().expect("test path"),
            )
        })
        .collect();

    assert!(entries.contains(&("fast", "tests/fast/lib.rs")));
    assert!(entries.contains(&("integration_tests", "tests/integration/lib.rs")));
    assert!(entries.contains(&("nightly", "tests/nightly/lib.rs")));

    let nextest: Value =
        toml::from_str(&ctx::read(".config/nextest.toml")).expect("nextest config should parse");
    let groups = nextest["test-groups"]
        .as_table()
        .expect("nextest test-groups should exist");

    assert_eq!(groups["fast"]["max-threads"].as_integer(), Some(8));
    assert_eq!(groups["integration"]["max-threads"].as_integer(), Some(4));
    assert_eq!(groups["nightly"]["max-threads"].as_integer(), Some(2));
    assert_eq!(
        nextest["profile"]["ci"]["junit"]["path"].as_str(),
        Some("tests/reports/nextest/junit.xml")
    );
    fs::dir("tests/reports/nextest");
}

#[test]
fn fast_shell_runner_executes_required_steps_and_respects_optional_gates() {
    let disabled = run_script(
        "scripts/testfast.sh",
        &[],
        &["geiger", "rudra", "audit"],
        &[],
    );

    assert!(disabled.stderr.is_empty());
    assert!(disabled.stdout.contains("==> nextest"));
    assert!(disabled.stdout.contains("==> clippy"));
    assert!(disabled.stdout.contains("==> rustfmt"));
    assert!(disabled.stdout.contains("==> skip geiger"));
    assert!(disabled.stdout.contains("==> skip rudra"));
    assert!(disabled.stdout.contains("==> skip audit"));
    assert_eq!(
        disabled.invocations,
        expected_invocations(
            &plan::fast(),
            ctx::root(),
            FAKE_HOST_TRIPLE,
            &[],
            &["geiger", "rudra", "audit"],
            &[],
        )
    );

    let enabled = run_script(
        "scripts/testfast.sh",
        &[
            ("HYPERCORE_ENABLE_GEIGER", "1"),
            ("HYPERCORE_ENABLE_RUDRA", "1"),
            ("HYPERCORE_ENABLE_AUDIT", "1"),
        ],
        &["geiger", "rudra", "audit"],
        &[],
    );

    assert!(enabled.stderr.is_empty());
    assert_eq!(
        enabled.invocations,
        expected_invocations(
            &plan::fast(),
            ctx::root(),
            FAKE_HOST_TRIPLE,
            &[
                "HYPERCORE_ENABLE_GEIGER",
                "HYPERCORE_ENABLE_RUDRA",
                "HYPERCORE_ENABLE_AUDIT",
            ],
            &["geiger", "rudra", "audit"],
            &[],
        )
    );
}

#[test]
fn powershell_fast_runner_stays_aligned_with_fast_tier() {
    fs::ordered(
        "scripts/run_tests_host.ps1",
        &[
            "$ErrorActionPreference = \"Stop\"",
            "function Get-HostTriple",
            "function Invoke-CargoStep",
            "function Invoke-OptionalCargoStep",
            "Invoke-CargoStep -Label \"Fast / nextest\"",
            "Invoke-CargoStep -Label \"Fast / clippy\"",
            "Invoke-CargoStep -Label \"Fast / rustfmt\"",
            "Invoke-OptionalCargoStep -Gate \"HYPERCORE_ENABLE_GEIGER\"",
            "Invoke-OptionalCargoStep -Gate \"HYPERCORE_ENABLE_RUDRA\"",
            "Invoke-OptionalCargoStep -Gate \"HYPERCORE_ENABLE_AUDIT\"",
        ],
    );

    fs::text("scripts/run_tests_host.ps1", ".config/nextest.toml");
    fs::text("scripts/run_tests_host.ps1", "-D\", \"warnings\"");
}

fn assert_tier_contract(tier: &TierPlan, expected_labels: &[&str]) {
    assert!(!tier.label.is_empty());
    assert_eq!(tier.commands.len(), expected_labels.len());

    let labels: Vec<_> = tier.commands.iter().map(|command| command.label).collect();
    assert_eq!(labels, expected_labels);

    let mut seen = HashSet::new();
    for command in &tier.commands {
        assert!(
            seen.insert(command.label),
            "duplicate command label: {}",
            command.label
        );
        assert!(
            !command.program.is_empty(),
            "missing program for {}",
            command.label
        );
        assert!(
            !command.workdir.is_empty(),
            "missing workdir for {}",
            command.label
        );
        assert!(
            !command.args.is_empty(),
            "missing args for {}",
            command.label
        );
        assert!(
            ctx::path(command.workdir).is_dir(),
            "missing workdir {} for {}",
            command.workdir,
            command.label
        );

        if let Some(gate) = command.gate {
            assert!(gate.starts_with("HYPERCORE_"), "invalid gate name: {gate}");
        }

        match command.availability {
            Availability::None => {}
            Availability::CargoSubcommand(subcommand) => {
                assert_eq!(command.program, "cargo");
                assert_eq!(command.args.first().copied(), Some(subcommand));
            }
            Availability::Binary(binary) => assert_eq!(command.program, binary),
        }

        for arg in command.args {
            assert!(!arg.is_empty(), "empty arg in {}", command.label);
            if is_repo_path(arg) {
                assert!(
                    ctx::path(arg).exists(),
                    "missing asset path {arg} for {}",
                    command.label
                );
            }
        }
    }
}

fn is_repo_path(arg: &str) -> bool {
    arg.contains('/') && !arg.starts_with("<")
}
