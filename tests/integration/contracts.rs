use crate::common::{ctx, fs, plan};

#[test]
fn integration_tier_commands_remain_explicit_and_gated() {
    let tier = plan::integration();
    let labels: Vec<_> = tier.commands.iter().map(|command| command.label).collect();

    assert_eq!(
        labels,
        vec![
            "nextest",
            "clippy",
            "kasan",
            "kmsan",
            "ubsan",
            "virtme",
            "cargofuzz",
        ]
    );
    assert_eq!(tier.named("nextest").gate, None);
    assert_eq!(tier.named("clippy").gate, None);
    assert_eq!(tier.named("kasan").gate, Some("AETHERCORE_RUN_KASAN"));
    assert_eq!(tier.named("kmsan").gate, Some("AETHERCORE_RUN_KMSAN"));
    assert_eq!(tier.named("ubsan").gate, Some("AETHERCORE_RUN_UBSAN"));
    assert_eq!(tier.named("virtme").gate, Some("AETHERCORE_RUN_VIRTME"));
    assert_eq!(tier.named("cargofuzz").gate, Some("AETHERCORE_RUN_FUZZ"));
}

#[test]
fn integration_xtask_runner_keeps_optional_tools_explicit() {
    fs::ordered(
        "xtask/src/commands/validation/test/tier.rs",
        &[
            "fn tier_specs(tier: &str, ci: bool, host: &str) -> Result<Vec<CommandSpec>> {",
            "test_consts::TIER_INTEGRATION => Ok(integration_specs(ci, host))",
            "fn integration_specs(ci: bool, host: &str) -> Vec<CommandSpec> {",
            "nextest_spec(\"integration_tests\", ci, host)",
            "host_cargo_test_spec(",
            "\"AETHERCORE_RUN_KASAN\"",
            "\"AETHERCORE_RUN_KMSAN\"",
            "\"AETHERCORE_RUN_UBSAN\"",
            "binary_spec(",
            "\"AETHERCORE_RUN_VIRTME\"",
            "\"AETHERCORE_RUN_FUZZ\"",
        ],
    );
    let body = ctx::read("xtask/src/commands/validation/test/tier.rs");
    assert!(body.contains("vng"));
    assert!(body.contains("host_rust_tests/Cargo.toml"));
    assert!(body.contains("host_tools/scheduler_host_tests/Cargo.toml"));
    assert!(body.contains("xagent/Cargo.toml"));
    assert!(body.contains("kernel_config_bytes"));
}
