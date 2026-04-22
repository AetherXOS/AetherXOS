use anyhow::{Result, bail};
use std::env;

use crate::constants::{cargo as cargo_consts, test as test_consts, tools};
use crate::utils::{cargo, process};

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
        bail!(
            "unknown test phase: {tier}. Supported: {:?}",
            test_consts::all_tiers()
        );
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
        availability: ToolAvailability::CargoSubcommand("nextest"),
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
            "-p".into(),
            "xtask".into(),
            "--".into(),
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
    std::process::Command::new(tools::CARGO)
        .args([subcommand, "--help"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn gate_enabled(name: &str) -> bool {
    env::var(name).map(|value| value == "1").unwrap_or(false)
}

#[cfg(test)]
#[path = "tier/tests.rs"]
mod tests;
