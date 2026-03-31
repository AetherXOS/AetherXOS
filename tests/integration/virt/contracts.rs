use crate::common::plan;

#[test]
fn virtme_plan_uses_explicit_runner_gate() {
    let plan = plan::integration();
    let cmd = plan.named("virtme");

    assert_eq!(cmd.program, "vng");
    assert_eq!(cmd.gate, Some("HYPERCORE_RUN_VIRTME"));
    assert_eq!(cmd.args, ["--version"]);
}
