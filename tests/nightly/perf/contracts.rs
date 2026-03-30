use crate::common::{fs, plan};

#[test]
fn flamegraph_plan_targets_scheduler_host_suite() {
    let cmd = plan::nightly().named("flamegraph");

    assert_eq!(cmd.program, "cargo");
    assert_eq!(cmd.gate, Some("HYPERCORE_RUN_FLAMEGRAPH"));
    assert!(cmd
        .args
        .contains(&"host_tools/scheduler_host_tests/Cargo.toml"));
    fs::file("host_tools/scheduler_host_tests/Cargo.toml");
}
