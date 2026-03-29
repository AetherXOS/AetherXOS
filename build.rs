/// HyperCore OS - modular build system.
/// Each subsystem owns its own validation under build_cfg/validators/.
/// See build_cfg/mod.rs for the orchestration pipeline.
#[path = "build_cfg/mod.rs"]
mod build_cfg;

fn main() {
    build_cfg::run();
}
