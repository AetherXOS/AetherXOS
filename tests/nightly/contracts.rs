use crate::common::tool::{expected_invocations, run_script, FAKE_HOST_TRIPLE};
use crate::common::{ctx, plan};

#[test]
fn nightly_tier_commands_remain_explicit_and_gated() {
    let tier = plan::nightly();
    let labels: Vec<_> = tier.commands.iter().map(|command| command.label).collect();

    assert_eq!(
        labels,
        vec![
            "nextest",
            "syzkaller",
            "tlaplus",
            "kani",
            "isabelle",
            "flamegraph"
        ]
    );
    assert_eq!(tier.named("nextest").gate, None);
    assert_eq!(
        tier.named("syzkaller").gate,
        Some("HYPERCORE_RUN_SYZKALLER")
    );
    assert_eq!(tier.named("tlaplus").gate, Some("HYPERCORE_RUN_TLAPLUS"));
    assert_eq!(tier.named("kani").gate, Some("HYPERCORE_RUN_KANI"));
    assert_eq!(tier.named("isabelle").gate, Some("HYPERCORE_RUN_ISABELLE"));
    assert_eq!(
        tier.named("flamegraph").gate,
        Some("HYPERCORE_RUN_FLAMEGRAPH")
    );
}

#[test]
fn nightly_shell_runner_executes_enabled_steps() {
    let run = run_script(
        "scripts/testnight.sh",
        &[
            ("HYPERCORE_RUN_KANI", "1"),
            ("HYPERCORE_RUN_TLAPLUS", "1"),
            ("HYPERCORE_RUN_ISABELLE", "1"),
            ("HYPERCORE_RUN_SYZKALLER", "1"),
            ("HYPERCORE_RUN_FLAMEGRAPH", "1"),
        ],
        &["kani", "flamegraph"],
        &["tlc", "isabelle", "syz-manager"],
    );

    assert!(run.stderr.is_empty());
    assert_eq!(
        run.invocations,
        expected_invocations(
            &plan::nightly(),
            ctx::root(),
            FAKE_HOST_TRIPLE,
            &[
                "HYPERCORE_RUN_KANI",
                "HYPERCORE_RUN_TLAPLUS",
                "HYPERCORE_RUN_ISABELLE",
                "HYPERCORE_RUN_SYZKALLER",
                "HYPERCORE_RUN_FLAMEGRAPH",
            ],
            &["kani", "flamegraph"],
            &["tlc", "isabelle", "syz-manager"],
        )
    );
}

#[test]
fn nightly_shell_runner_skips_unavailable_optional_tools() {
    let run = run_script(
        "scripts/testnight.sh",
        &[
            ("HYPERCORE_RUN_KANI", "1"),
            ("HYPERCORE_RUN_TLAPLUS", "1"),
            ("HYPERCORE_RUN_ISABELLE", "1"),
            ("HYPERCORE_RUN_SYZKALLER", "1"),
            ("HYPERCORE_RUN_FLAMEGRAPH", "1"),
        ],
        &[],
        &[],
    );

    assert!(run.stdout.contains("==> skip tlaplus"));
    assert!(run.stdout.contains("==> skip isabelle"));
    assert!(run.stdout.contains("==> skip syzkaller"));
    assert!(run.stdout.contains("==> skip kani"));
    assert!(run.stdout.contains("==> skip flamegraph"));
    assert_eq!(
        run.invocations,
        expected_invocations(
            &plan::nightly(),
            ctx::root(),
            FAKE_HOST_TRIPLE,
            &[
                "HYPERCORE_RUN_KANI",
                "HYPERCORE_RUN_TLAPLUS",
                "HYPERCORE_RUN_ISABELLE",
                "HYPERCORE_RUN_SYZKALLER",
                "HYPERCORE_RUN_FLAMEGRAPH",
            ],
            &[],
            &[],
        )
    );
}
