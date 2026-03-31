use crate::common::tool::{Availability, CommandPlan, TierPlan};

const TEST_FEATURES: &[&str] = &["kernel_test_mode", "vfs", "drivers"];
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

const FASTARGS: &[&str] = &[
    "nextest",
    "run",
    "--config-file",
    ".config/nextest.toml",
    "--target",
    "<host>",
    "--features",
    "kernel_test_mode,vfs,drivers",
    "--test",
    "fast",
];
const CLIPPYARGS: &[&str] = &[
    "clippy",
    "--manifest-path",
    "Cargo.toml",
    "--lib",
    "--target",
    "<host>",
    "--features",
    "kernel_test_mode,vfs,drivers",
    "--",
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
const FMTARGS: &[&str] = &["fmt", "--manifest-path", "Cargo.toml", "--all", "--check"];
const GEIGERARGS: &[&str] = &[
    "geiger",
    "--manifest-path",
    "Cargo.toml",
    "--all-targets",
    "--target",
    "<host>",
    "--features",
    "kernel_test_mode,vfs,drivers",
];
const RUDRAARGS: &[&str] = &[
    "rudra",
    "--manifest-path",
    "Cargo.toml",
    "--all-targets",
    "--target",
    "<host>",
    "--features",
    "kernel_test_mode,vfs,drivers",
];
const AUDITARGS: &[&str] = &["audit"];

const INTARGS: &[&str] = &[
    "nextest",
    "run",
    "--config-file",
    ".config/nextest.toml",
    "--target",
    "<host>",
    "--features",
    "kernel_test_mode,vfs,drivers",
    "--test",
    "integration_tests",
];
const FUZZARGS: &[&str] = &[
    "fuzz",
    "build",
    "kernel_config_bytes",
    "--manifest-path",
    "Cargo.toml",
];
const KASANARGS: &[&str] = &[
    "test",
    "--manifest-path",
    "host_rust_tests/Cargo.toml",
    "--tests",
];
const KMSANARGS: &[&str] = &[
    "test",
    "--manifest-path",
    "host_tools/scheduler_host_tests/Cargo.toml",
    "--tests",
];
const UBSANARGS: &[&str] = &["test", "--manifest-path", "agent/Cargo.toml", "--tests"];
const VIRTARGS: &[&str] = &["--version"];

const NIGHTARGS: &[&str] = &[
    "nextest",
    "run",
    "--config-file",
    ".config/nextest.toml",
    "--target",
    "<host>",
    "--features",
    "kernel_test_mode,vfs,drivers",
    "--test",
    "nightly",
];
const KANIARGS: &[&str] = &["kani", "--manifest-path", "formal/kani/Cargo.toml"];
const TLAARGS: &[&str] = &["formal/tla/KernelConfigOverrides.tla"];
const ISABELLEARGS: &[&str] = &["build", "-D", "formal/isabelle"];
const SYZARGS: &[&str] = &["-config", "formal/syzkaller/hypercore.cfg"];
const FLAMEARGS: &[&str] = &[
    "flamegraph",
    "--manifest-path",
    "host_tools/scheduler_host_tests/Cargo.toml",
    "--test",
    "scheduler_runtime",
];

pub fn test_feature_args() -> &'static [&'static str] {
    TEST_FEATURES
}

pub fn clippy_lint_args() -> &'static [&'static str] {
    CLIPPY_LINT_ARGS
}

pub fn fast() -> TierPlan {
    TierPlan {
        label: "fast",
        commands: vec![
            cargo_step("nextest", ".", FASTARGS),
            cargo_step("clippy", ".", CLIPPYARGS),
            cargo_step("rustfmt", ".", FMTARGS),
            cargo_optional_step(
                "geiger",
                ".",
                GEIGERARGS,
                "HYPERCORE_ENABLE_GEIGER",
                Availability::CargoSubcommand("geiger"),
            ),
            cargo_optional_step(
                "rudra",
                ".",
                RUDRAARGS,
                "HYPERCORE_ENABLE_RUDRA",
                Availability::CargoSubcommand("rudra"),
            ),
            cargo_optional_step(
                "audit",
                ".",
                AUDITARGS,
                "HYPERCORE_ENABLE_AUDIT",
                Availability::CargoSubcommand("audit"),
            ),
        ],
    }
}

pub fn integration() -> TierPlan {
    TierPlan {
        label: "integration",
        commands: vec![
            cargo_step("nextest", ".", INTARGS),
            cargo_step("clippy", ".", CLIPPYARGS),
            cargo_optional_step(
                "kasan",
                ".",
                KASANARGS,
                "HYPERCORE_RUN_KASAN",
                Availability::None,
            ),
            cargo_optional_step(
                "kmsan",
                ".",
                KMSANARGS,
                "HYPERCORE_RUN_KMSAN",
                Availability::None,
            ),
            cargo_optional_step(
                "ubsan",
                ".",
                UBSANARGS,
                "HYPERCORE_RUN_UBSAN",
                Availability::None,
            ),
            binary_optional_step("virtme", ".", "vng", VIRTARGS, "HYPERCORE_RUN_VIRTME"),
            cargo_optional_step(
                "cargofuzz",
                "fuzz",
                FUZZARGS,
                "HYPERCORE_RUN_FUZZ",
                Availability::CargoSubcommand("fuzz"),
            ),
        ],
    }
}

pub fn nightly() -> TierPlan {
    TierPlan {
        label: "nightly",
        commands: vec![
            cargo_step("nextest", ".", NIGHTARGS),
            cargo_step("clippy", ".", CLIPPYARGS),
            binary_optional_step(
                "syzkaller",
                ".",
                "syz-manager",
                SYZARGS,
                "HYPERCORE_RUN_SYZKALLER",
            ),
            binary_optional_step("tlaplus", ".", "tlc", TLAARGS, "HYPERCORE_RUN_TLAPLUS"),
            cargo_optional_step(
                "kani",
                ".",
                KANIARGS,
                "HYPERCORE_RUN_KANI",
                Availability::CargoSubcommand("kani"),
            ),
            binary_optional_step(
                "isabelle",
                ".",
                "isabelle",
                ISABELLEARGS,
                "HYPERCORE_RUN_ISABELLE",
            ),
            cargo_optional_step(
                "flamegraph",
                ".",
                FLAMEARGS,
                "HYPERCORE_RUN_FLAMEGRAPH",
                Availability::CargoSubcommand("flamegraph"),
            ),
        ],
    }
}

fn cargo_step(
    label: &'static str,
    workdir: &'static str,
    args: &'static [&'static str],
) -> CommandPlan {
    CommandPlan {
        label,
        workdir,
        program: "cargo",
        args,
        gate: None,
        availability: Availability::None,
    }
}

fn cargo_optional_step(
    label: &'static str,
    workdir: &'static str,
    args: &'static [&'static str],
    gate: &'static str,
    availability: Availability,
) -> CommandPlan {
    CommandPlan {
        label,
        workdir,
        program: "cargo",
        args,
        gate: Some(gate),
        availability,
    }
}

fn binary_optional_step(
    label: &'static str,
    workdir: &'static str,
    program: &'static str,
    args: &'static [&'static str],
    gate: &'static str,
) -> CommandPlan {
    CommandPlan {
        label,
        workdir,
        program,
        args,
        gate: Some(gate),
        availability: Availability::Binary(program),
    }
}
