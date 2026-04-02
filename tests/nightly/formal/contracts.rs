use crate::common::{fs, plan};

#[test]
fn formal_assets_and_plans_stay_registered() {
    let tier = plan::nightly();

    assert_eq!(tier.named("kani").program, "cargo");
    assert!(tier.named("kani").args.contains(&"formal/kani/Cargo.toml"));
    fs::file("formal/kani/Cargo.toml");
    fs::file("formal/kani/src/lib.rs");
    fs::text("formal/kani/src/lib.rs", "#[kani::proof]");

    assert_eq!(tier.named("tlaplus").program, "tlc");
    assert!(
        tier.named("tlaplus")
            .args
            .contains(&"formal/tla/KernelConfigOverrides.tla")
    );
    fs::file("formal/tla/KernelConfigOverrides.tla");
    fs::file("formal/tla/KernelConfigOverrides.cfg");
    fs::text("formal/tla/KernelConfigOverrides.tla", "HistoryInvariant");

    assert_eq!(tier.named("isabelle").program, "isabelle");
    assert!(tier.named("isabelle").args.contains(&"formal/isabelle"));
    fs::file("formal/isabelle/ROOT");
    fs::file("formal/isabelle/Kernel_Config.thy");
    fs::text(
        "formal/isabelle/Kernel_Config.thy",
        "telemetry_history_len_positive",
    );
}
