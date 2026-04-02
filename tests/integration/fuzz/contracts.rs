use crate::common::{fs, plan};

#[test]
fn cargo_fuzz_assets_and_plan_stay_aligned() {
    let plan = plan::integration();
    let cmd = plan.named("cargofuzz");

    assert_eq!(cmd.program, "cargo");
    assert_eq!(cmd.workdir, "fuzz");
    assert_eq!(cmd.gate, Some("AETHERCORE_RUN_FUZZ"));
    assert_eq!(cmd.args, ["fuzz", "build", "kernel_config_bytes", "--manifest-path", "Cargo.toml"]);
    fs::file("fuzz/Cargo.toml");
    fs::file("fuzz/fuzz_targets/kernel_config_bytes.rs");
    fs::text("fuzz/Cargo.toml", "cargo-fuzz = true");
}
