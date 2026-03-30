use crate::common::{fs, plan};

#[test]
fn syzkaller_plan_targets_repo_config() {
    let cmd = plan::nightly().named("syzkaller");

    assert_eq!(cmd.program, "syz-manager");
    assert_eq!(cmd.gate, Some("HYPERCORE_RUN_SYZKALLER"));
    assert!(cmd.args.contains(&"formal/syzkaller/hypercore.cfg"));
    fs::file("formal/syzkaller/hypercore.cfg");
    fs::text("formal/syzkaller/hypercore.cfg", "hypercore.img");
}
