use anyhow::{bail, Result};
use std::env;

use crate::utils::{cargo, process};
use crate::constants::{cargo as cargo_consts, test as test_consts};

const CLIPPY_LINT_ARGS: &[&str] = &[
    "-A",
    "warnings",
    "-A",
    "unused",
    "-A",
    "dead_code",
    "-A",
    "unused_imports",
    "-A",
    "unused_variables",
    "-A",
    "unused_mut",
    "-A",
    "unsafe_op_in_unsafe_fn",
    "-A",
    "clippy::all",
];

#[derive(Clone, Debug, PartialEq, Eq)]
enum ToolAvailability {
    None,
    CargoSubcommand(&'static str),
    Binary(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CommandSpec {
    label: &'static str,
    program: &'static str,
    args: Vec<String>,
    gate: Option<&'static str>,
    availability: ToolAvailability,
}

pub fn run(tier: &str, ci: bool) -> Result<()> {
    let host = cargo::detect_host_triple()?;
    let specs = tier_specs(tier, ci, &host)?;

    for spec in specs {
        run_spec(&spec)?;
    }

    Ok(())
}

pub fn run_all(ci: bool) -> Result<()> {
    for tier in test_consts::all_tiers() {
        run(tier, ci)?;
    }
    Ok(())
}

fn tier_specs(tier: &str, ci: bool, host: &str) -> Result<Vec<CommandSpec>> {
    if !test_consts::is_valid_tier(tier) {
        bail!("unknown tier: {tier}. Supported: {:?}", test_consts::all_tiers());
    }
    match tier {
        test_consts::TIER_FAST => Ok(fast_specs(ci, host)),
        test_consts::TIER_INTEGRATION => Ok(integration_specs(ci, host)),
        test_consts::TIER_NIGHTLY => Ok(nightly_specs(ci, host)),
        _ => unreachable!(),
    }
}

fn fast_specs(ci: bool, host: &str) -> Vec<CommandSpec> {
    vec![
        nextest_spec("fast", ci, host),
        clippy_spec(host),
        rustfmt_spec(),
        cargo_subcommand_spec(
            "geiger",
            "AETHERCORE_ENABLE_GEIGER",
            vec![
                "geiger".into(),
                cargo_consts::ARG_MANIFEST_PATH.into(),
                cargo_consts::MANIFEST_FILE.into(),
                "--all-targets".into(),
                cargo_consts::ARG_TARGET.into(),
                host.into(),
                cargo_consts::ARG_FEATURES.into(),
                test_consts::TEST_FEATURES.into(),
            ],
        ),
        cargo_subcommand_spec(
            "rudra",
            "AETHERCORE_ENABLE_RUDRA",
            vec![
                "rudra".into(),
                "--manifest-path".into(),
                "Cargo.toml".into(),
                "--all-targets".into(),
                "--target".into(),
                host.into(),
                "--features".into(),
                test_consts::TEST_FEATURES.into(),
            ],
        ),
        cargo_subcommand_spec("audit", "AETHERCORE_ENABLE_AUDIT", vec!["audit".into()]),
    ]
}

fn integration_specs(ci: bool, host: &str) -> Vec<CommandSpec> {
    vec![
        nextest_spec("integration_tests", ci, host),
        clippy_spec(host),
        host_cargo_test_spec(
            "kasan",
            "AETHERCORE_RUN_KASAN",
            "host_rust_tests/Cargo.toml",
            host,
        ),
        host_cargo_test_spec(
            "kmsan",
            "AETHERCORE_RUN_KMSAN",
            "host_tools/scheduler_host_tests/Cargo.toml",
            host,
        ),
        host_cargo_test_spec("ubsan", "AETHERCORE_RUN_UBSAN", "xagent/Cargo.toml", host),
        binary_spec(
            "virtme",
            "vng",
            "AETHERCORE_RUN_VIRTME",
            vec!["--version".into()],
        ),
        cargo_subcommand_spec(
            "cargofuzz",
            "AETHERCORE_RUN_FUZZ",
            vec![
                "fuzz".into(),
                "build".into(),
                "kernel_config_bytes".into(),
                "--manifest-path".into(),
                "fuzz/Cargo.toml".into(),
            ],
        ),
    ]
}

fn nightly_specs(ci: bool, host: &str) -> Vec<CommandSpec> {
    vec![
        nextest_spec("nightly", ci, host),
        clippy_spec(host),
        binary_spec(
            "syzkaller",
            "syz-manager",
            "AETHERCORE_RUN_SYZKALLER",
            vec!["-config".into(), "formal/syzkaller/aethercore.cfg".into()],
        ),
        binary_spec(
            "tlaplus",
            "tlc",
            "AETHERCORE_RUN_TLAPLUS",
            vec!["formal/tla/KernelConfigOverrides.tla".into()],
        ),
        cargo_subcommand_spec(
            "kani",
            "AETHERCORE_RUN_KANI",
            vec![
                "kani".into(),
                "--manifest-path".into(),
                "formal/kani/Cargo.toml".into(),
            ],
        ),
        binary_spec(
            "isabelle",
            "isabelle",
            "AETHERCORE_RUN_ISABELLE",
            vec!["build".into(), "-D".into(), "formal/isabelle".into()],
        ),
        cargo_subcommand_spec(
            "flamegraph",
            "AETHERCORE_RUN_FLAMEGRAPH",
            vec![
                "flamegraph".into(),
                "--manifest-path".into(),
                "host_tools/scheduler_host_tests/Cargo.toml".into(),
                "--test".into(),
                "scheduler_runtime".into(),
            ],
        ),
    ]
}

fn nextest_spec(test_name: &'static str, ci: bool, host: &str) -> CommandSpec {
    let mut args = vec![
        "nextest".into(),
        cargo_consts::CMD_RUN.into(),
        "--config-file".into(),
        ".config/nextest.toml".into(),
    ];
    if ci {
        args.extend(["--profile".into(), "ci".into()]);
    }
    args.extend([
        "--target".into(),
        host.into(),
        "--features".into(),
        test_consts::TEST_FEATURES.into(),
        "--test".into(),
        test_name.into(),
    ]);

    CommandSpec {
        label: "nextest",
        program: crate::constants::tools::CARGO,
        args,
        gate: None,
        availability: ToolAvailability::None,
    }
}

fn clippy_spec(host: &str) -> CommandSpec {
    let mut args = vec![
        "clippy".into(),
        "--manifest-path".into(),
        "Cargo.toml".into(),
        "--lib".into(),
        "--target".into(),
        host.into(),
        "--features".into(),
        test_consts::TEST_FEATURES.into(),
        "--".into(),
    ];
    args.extend(CLIPPY_LINT_ARGS.iter().map(|arg| (*arg).to_string()));

    CommandSpec {
        label: "clippy",
        program: "cargo",
        args,
        gate: None,
        availability: ToolAvailability::None,
    }
}

fn rustfmt_spec() -> CommandSpec {
    CommandSpec {
        label: "rustfmt",
        program: "cargo",
        args: vec![
            "fmt".into(),
            "--manifest-path".into(),
            "Cargo.toml".into(),
            "--all".into(),
            "--check".into(),
        ],
        gate: None,
        availability: ToolAvailability::None,
    }
}

fn host_cargo_test_spec(
    label: &'static str,
    gate: &'static str,
    manifest_path: &'static str,
    host: &str,
) -> CommandSpec {
    CommandSpec {
        label,
        program: crate::constants::tools::CARGO,
        args: vec![
            cargo_consts::CMD_TEST.into(),
            cargo_consts::ARG_MANIFEST_PATH.into(),
            manifest_path.into(),
            cargo_consts::ARG_TARGET.into(),
            host.into(),
            "--tests".into(),
        ],
        gate: Some(gate),
        availability: ToolAvailability::None,
    }
}

fn cargo_subcommand_spec(
    label: &'static str,
    gate: &'static str,
    args: Vec<String>,
) -> CommandSpec {
    CommandSpec {
        label,
        program: crate::constants::tools::CARGO,
        availability: ToolAvailability::CargoSubcommand(label_for_probe(label)),
        gate: Some(gate),
        args,
    }
}

fn binary_spec(
    label: &'static str,
    program: &'static str,
    gate: &'static str,
    args: Vec<String>,
) -> CommandSpec {
    CommandSpec {
        label,
        program,
        args,
        gate: Some(gate),
        availability: ToolAvailability::Binary(program),
    }
}

fn label_for_probe(label: &'static str) -> &'static str {
    match label {
        "cargofuzz" => "fuzz",
        other => other,
    }
}

fn run_spec(spec: &CommandSpec) -> Result<()> {
    match spec.availability {
        ToolAvailability::None => run_gated_or_required(spec),
        ToolAvailability::CargoSubcommand(subcommand) => {
            if cargo_subcommand_available(subcommand) {
                run_gated_or_required(spec)
            } else {
                println!("==> skip {}", spec.label);
                Ok(())
            }
        }
        ToolAvailability::Binary(binary) => {
            if process::which(binary) {
                run_gated_or_required(spec)
            } else {
                println!("==> skip {}", spec.label);
                Ok(())
            }
        }
    }
}

fn run_gated_or_required(spec: &CommandSpec) -> Result<()> {
    if spec.gate.is_some_and(|gate| !gate_enabled(gate)) {
        println!("==> skip {}", spec.label);
        return Ok(());
    }

    println!("==> {}", spec.label);
    if spec.label == "nextest" {
        return process::run_checked_with_env_owned(
            spec.program,
            &spec.args,
            &[("RUSTFLAGS", "-A warnings")],
        );
    }
    process::run_checked_owned(spec.program, &spec.args)
}

fn cargo_subcommand_available(subcommand: &str) -> bool {
    std::process::Command::new("cargo")
        .args([subcommand, "--help"])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn gate_enabled(name: &str) -> bool {
    env::var(name).map(|value| value == "1").unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integration_gated_host_tests_pin_host_target() {
        let specs = integration_specs(false, "x86_64-unknown-linux-gnu");

        let kasan = specs.iter().find(|spec| spec.label == "kasan").unwrap();
        let kmsan = specs.iter().find(|spec| spec.label == "kmsan").unwrap();
        let ubsan = specs.iter().find(|spec| spec.label == "ubsan").unwrap();

        assert_eq!(kasan.program, "cargo");
        assert_eq!(kmsan.program, "cargo");
        assert_eq!(ubsan.program, "cargo");
        assert!(kasan
            .args
            .windows(2)
            .any(|pair| pair == ["--target", "x86_64-unknown-linux-gnu"]));
        assert!(kmsan
            .args
            .windows(2)
            .any(|pair| pair == ["--target", "x86_64-unknown-linux-gnu"]));
        assert!(ubsan
            .args
            .windows(2)
            .any(|pair| pair == ["--target", "x86_64-unknown-linux-gnu"]));
    }

    #[test]
    fn nextest_ci_profile_is_opt_in_and_host_scoped() {
        let dev = nextest_spec("fast", false, "aarch64-unknown-linux-gnu");
        let ci = nextest_spec("fast", true, "aarch64-unknown-linux-gnu");

        assert!(!dev.args.windows(2).any(|pair| pair == ["--profile", "ci"]));
        assert!(ci.args.windows(2).any(|pair| pair == ["--profile", "ci"]));
        assert!(ci
            .args
            .windows(2)
            .any(|pair| pair == ["--target", "aarch64-unknown-linux-gnu"]));
    }

    #[test]
    fn cargo_subcommand_probe_names_match_tooling_contracts() {
        assert_eq!(label_for_probe("geiger"), "geiger");
        assert_eq!(label_for_probe("kani"), "kani");
        assert_eq!(label_for_probe("cargofuzz"), "fuzz");
    }

    #[test]
    fn run_all_accepts_core_tiers_in_order() {
        let tiers = ["fast", "integration", "nightly"];
        let host = "x86_64-unknown-linux-gnu";

        let labels: Vec<_> = tiers
            .into_iter()
            .flat_map(|tier| tier_specs(tier, false, host).unwrap())
            .map(|spec| spec.label)
            .collect();

        assert_eq!(labels.first().copied(), Some("nextest"));
        assert!(labels.len() >= 3);
        assert_eq!(tier_specs("fast", false, host).unwrap()[0].label, "nextest");
        assert_eq!(
            tier_specs("integration", false, host).unwrap()[0].label,
            "nextest"
        );
        assert_eq!(
            tier_specs("nightly", false, host).unwrap()[0].label,
            "nextest"
        );
    }
}
