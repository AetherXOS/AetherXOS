use crate::common::tool::{expected_invocations, run_script, FAKE_HOST_TRIPLE};
use crate::common::{ctx, plan};

#[test]
fn integration_tier_commands_remain_explicit_and_gated() {
    let tier = plan::integration();
    let labels: Vec<_> = tier.commands.iter().map(|command| command.label).collect();

    assert_eq!(
        labels,
        vec!["nextest", "kasan", "kmsan", "ubsan", "virtme", "cargofuzz"]
    );
    assert_eq!(tier.named("nextest").gate, None);
    assert_eq!(tier.named("kasan").gate, Some("HYPERCORE_RUN_KASAN"));
    assert_eq!(tier.named("kmsan").gate, Some("HYPERCORE_RUN_KMSAN"));
    assert_eq!(tier.named("ubsan").gate, Some("HYPERCORE_RUN_UBSAN"));
    assert_eq!(tier.named("virtme").gate, Some("HYPERCORE_RUN_VIRTME"));
    assert_eq!(tier.named("cargofuzz").gate, Some("HYPERCORE_RUN_FUZZ"));
}

#[test]
fn integration_shell_runner_executes_enabled_steps() {
    let run = run_script(
        "scripts/testint.sh",
        &[
            ("HYPERCORE_RUN_KASAN", "1"),
            ("HYPERCORE_RUN_KMSAN", "1"),
            ("HYPERCORE_RUN_UBSAN", "1"),
            ("HYPERCORE_RUN_VIRTME", "1"),
            ("HYPERCORE_RUN_FUZZ", "1"),
        ],
        &["fuzz"],
        &["vng"],
    );

    assert!(run.stderr.is_empty());
    assert_eq!(
        run.invocations,
        expected_invocations(
            &plan::integration(),
            ctx::root(),
            FAKE_HOST_TRIPLE,
            &[
                "HYPERCORE_RUN_KASAN",
                "HYPERCORE_RUN_KMSAN",
                "HYPERCORE_RUN_UBSAN",
                "HYPERCORE_RUN_VIRTME",
                "HYPERCORE_RUN_FUZZ",
            ],
            &["fuzz"],
            &["vng"],
        )
    );
}

#[test]
fn integration_shell_runner_skips_unavailable_optional_tools() {
    let run = run_script(
        "scripts/testint.sh",
        &[
            ("HYPERCORE_RUN_KASAN", "1"),
            ("HYPERCORE_RUN_KMSAN", "1"),
            ("HYPERCORE_RUN_UBSAN", "1"),
            ("HYPERCORE_RUN_VIRTME", "1"),
            ("HYPERCORE_RUN_FUZZ", "1"),
        ],
        &[],
        &[],
    );

    assert!(run.stdout.contains("==> skip virtme"));
    assert!(run.stdout.contains("==> skip cargofuzz"));
    assert_eq!(
        run.invocations,
        expected_invocations(
            &plan::integration(),
            ctx::root(),
            FAKE_HOST_TRIPLE,
            &[
                "HYPERCORE_RUN_KASAN",
                "HYPERCORE_RUN_KMSAN",
                "HYPERCORE_RUN_UBSAN",
                "HYPERCORE_RUN_VIRTME",
                "HYPERCORE_RUN_FUZZ",
            ],
            &[],
            &[],
        )
    );
}
