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
    assert_eq!(
        tier.named("syzkaller").gate,
        Some("AETHERCORE_RUN_SYZKALLER")
    );
    assert_eq!(tier.named("tlaplus").gate, Some("AETHERCORE_RUN_TLAPLUS"));
    assert_eq!(tier.named("kani").gate, Some("AETHERCORE_RUN_KANI"));
    assert_eq!(tier.named("isabelle").gate, Some("AETHERCORE_RUN_ISABELLE"));
    assert_eq!(
        tier.named("flamegraph").gate,
        Some("AETHERCORE_RUN_FLAMEGRAPH")
    );
}

#[test]
fn nightly_xtask_runner_keeps_optional_provers_explicit() {
    fs::ordered(
        "xtask/src/commands/validation/test/tier.rs",
        &[
            "fn tier_specs(tier: &str, ci: bool, host: &str) -> Result<Vec<CommandSpec>> {",
            "test_consts::TIER_NIGHTLY => Ok(nightly_specs(ci, host))",
            "fn nightly_specs(ci: bool, host: &str) -> Vec<CommandSpec> {",
            "nextest_spec(\"nightly\", ci, host)",
            "binary_spec(",
            "\"syzkaller\"",
            "\"AETHERCORE_RUN_SYZKALLER\"",
            "\"AETHERCORE_RUN_TLAPLUS\"",
            "\"AETHERCORE_RUN_KANI\"",
            "\"AETHERCORE_RUN_ISABELLE\"",
            "\"AETHERCORE_RUN_FLAMEGRAPH\"",
        ],
    );
    let body = ctx::read("xtask/src/commands/validation/test/tier.rs");
    assert!(body.contains("formal/tla/KernelConfigOverrides.tla"));
    assert!(body.contains("formal/isabelle"));
    assert!(body.contains("formal/syzkaller/aethercore.cfg"));
    assert!(body.contains("formal/kani/Cargo.toml"));
}
