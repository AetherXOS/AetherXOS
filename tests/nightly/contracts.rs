use crate::common::{ctx, fs, plan};

#[test]
fn nightly_tier_commands_remain_explicit_and_gated() {
    let tier = plan::nightly();
    let labels: Vec<_> = tier.commands.iter().map(|command| command.label).collect();

    assert_eq!(
        labels,
        vec![
            "nextest",
            "clippy",
            "syzkaller",
            "tlaplus",
            "kani",
            "isabelle",
            "flamegraph",
        ]
    );
    assert_eq!(tier.named("nextest").gate, None);
    assert_eq!(tier.named("clippy").gate, None);
    assert_eq!(tier.named("syzkaller").gate, Some("HYPERCORE_RUN_SYZKALLER"));
    assert_eq!(tier.named("tlaplus").gate, Some("HYPERCORE_RUN_TLAPLUS"));
    assert_eq!(tier.named("kani").gate, Some("HYPERCORE_RUN_KANI"));
    assert_eq!(tier.named("isabelle").gate, Some("HYPERCORE_RUN_ISABELLE"));
    assert_eq!(tier.named("flamegraph").gate, Some("HYPERCORE_RUN_FLAMEGRAPH"));
}

#[test]
fn nightly_xtask_runner_keeps_optional_provers_explicit() {
    fs::ordered(
        "xtask/src/commands/validation/test/tier.rs",
        &[
            "fn run_nightly(ci: bool) -> Result<()> {",
            "run_nextest(\"nightly\", ci)?;",
            "run_clippy()?;",
            "run_optional_binary(\"HYPERCORE_RUN_SYZKALLER\"",
            "run_optional_binary(\"HYPERCORE_RUN_TLAPLUS\"",
            "run_optional_cargo_subcommand(\"HYPERCORE_RUN_KANI\"",
            "run_optional_binary(\"HYPERCORE_RUN_ISABELLE\"",
            "run_optional_cargo_subcommand(\"HYPERCORE_RUN_FLAMEGRAPH\"",
        ],
    );
    let body = ctx::read("xtask/src/commands/validation/test/tier.rs");
    assert!(body.contains("formal/tla/KernelConfigOverrides.tla"));
    assert!(body.contains("formal/isabelle"));
    assert!(body.contains("formal/syzkaller/hypercore.cfg"));
    assert!(body.contains("formal/kani/Cargo.toml"));
}
