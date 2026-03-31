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
    assert_eq!(tier.named("kasan").gate, Some("HYPERCORE_RUN_KASAN"));
    assert_eq!(tier.named("kmsan").gate, Some("HYPERCORE_RUN_KMSAN"));
    assert_eq!(tier.named("ubsan").gate, Some("HYPERCORE_RUN_UBSAN"));
    assert_eq!(tier.named("virtme").gate, Some("HYPERCORE_RUN_VIRTME"));
    assert_eq!(tier.named("cargofuzz").gate, Some("HYPERCORE_RUN_FUZZ"));
}

#[test]
fn integration_xtask_runner_keeps_optional_tools_explicit() {
    fs::ordered(
        "xtask/src/commands/validation/test/tier.rs",
        &[
            "fn run_integration(ci: bool) -> Result<()> {",
            "run_nextest(\"integration_tests\", ci)?;",
            "run_clippy()?;",
            "run_gate(\"HYPERCORE_RUN_KASAN\"",
            "run_gate(\"HYPERCORE_RUN_KMSAN\"",
            "run_gate(\"HYPERCORE_RUN_UBSAN\"",
            "run_optional_binary(\"HYPERCORE_RUN_VIRTME\"",
            "run_optional_cargo_subcommand(\"HYPERCORE_RUN_FUZZ\"",
        ],
    );
    let body = ctx::read("xtask/src/commands/validation/test/tier.rs");
    assert!(body.contains("vng"));
    assert!(body.contains("host_rust_tests/Cargo.toml"));
    assert!(body.contains("host_tools/scheduler_host_tests/Cargo.toml"));
    assert!(body.contains("agent/Cargo.toml"));
    assert!(body.contains("kernel_config_bytes"));
}
