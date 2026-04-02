use crate::common::{fs, plan};

#[test]
fn syzkaller_plan_targets_repo_config() {
    let tier = plan::nightly();
    let cmd = tier.named("syzkaller");

    assert_eq!(cmd.program, "syz-manager");
    assert_eq!(cmd.gate, Some("AETHERCORE_RUN_SYZKALLER"));
    assert!(cmd.args.contains(&"formal/syzkaller/aethercore.cfg"));
    fs::file("formal/syzkaller/aethercore.cfg");
    fs::text("formal/syzkaller/aethercore.cfg", "aethercore.img");
}
