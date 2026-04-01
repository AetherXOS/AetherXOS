use std::collections::HashSet;

use crate::common::tool::{Availability, TierPlan};
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
        &[
            "nextest",
            "clippy",
            "kasan",
            "kmsan",
            "ubsan",
            "virtme",
            "cargofuzz",
        ],
    );
    assert_tier_contract(
        &plan::nightly(),
        &[
            "nextest",
            "clippy",
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
fn xtask_tier_runner_is_the_single_ci_entrypoint() {
    fs::file("xtask/src/commands/validation/test/tier.rs");
    fs::ordered(
        "xtask/src/cli.rs",
        &[
            "pub enum TestAction",
            "Tier {",
            "tier: String",
            "ci: bool",
        ],
    );
    fs::ordered(
        "xtask/src/commands/validation/test/mod.rs",
        &[
            "pub mod tier;",
            "TestAction::Tier { tier, ci } => tier::run(tier, *ci)",
        ],
    );
    fs::ordered(
        "xtask/src/commands/validation/test/tier.rs",
        &[
            "const TEST_FEATURES: &str = \"kernel_test_mode,vfs,drivers\";",
            "pub fn run(tier: &str, ci: bool) -> Result<()> {",
            "\"fast\" => run_fast(ci)",
            "\"integration\" => run_integration(ci)",
            "\"nightly\" => run_nightly(ci)",
        ],
    );
}

#[test]
fn reusable_workflows_drive_tier_commands() {
    fs::file(".github/workflows/tier-reusable.yml");
    for workflow in [
        ".github/workflows/x64-fast.yml",
        ".github/workflows/x64-integration.yml",
        ".github/workflows/x64-nightly.yml",
        ".github/workflows/arm64-fast.yml",
        ".github/workflows/arm64-integration.yml",
        ".github/workflows/arm64-nightly.yml",
    ] {
        fs::text(workflow, "uses: ./.github/workflows/tier-reusable.yml");
        fs::text(workflow, "tier:");
    }
    fs::text(
        ".github/workflows/tier-reusable.yml",
        "cargo run -p xtask --target-dir target/xtask -- test tier ${{ inputs.tier }} --ci",
    );
    fs::text(".github/workflows/x64-integration.yml", "cron: '0 16 * * *'");
    fs::text(".github/workflows/arm64-integration.yml", "cron: '0 16 * * *'");
    fs::text(".github/workflows/x64-nightly.yml", "cron: '0 16 * * 6,0'");
    fs::text(".github/workflows/arm64-nightly.yml", "cron: '0 16 * * 6,0'");
    assert!(ctx::path(".github/workflows/linux-host-e2e.yml").exists());
}

#[test]
fn legacy_shell_tier_entrypoints_are_removed() {
    assert!(!ctx::path("scripts/testfast.sh").exists());
    assert!(!ctx::path("scripts/testint.sh").exists());
    assert!(!ctx::path("scripts/testnight.sh").exists());
    assert!(!ctx::path("scripts/run_tests_host.ps1").exists());
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
