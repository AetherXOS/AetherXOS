use crate::common::{fs, plan};

#[test]
fn sanitizer_plans_stay_pinned_to_dedicated_crates() {
    let tier = plan::integration();

    assert_eq!(tier.named("kasan").program, "cargo");
    assert_eq!(tier.named("kasan").gate, Some("AETHERCORE_RUN_KASAN"));
    assert!(tier
        .named("kasan")
        .args
        .contains(&"host_rust_tests/Cargo.toml"));

    assert_eq!(tier.named("kmsan").program, "cargo");
    assert_eq!(tier.named("kmsan").gate, Some("AETHERCORE_RUN_KMSAN"));
    assert!(tier
        .named("kmsan")
        .args
        .contains(&"host_tools/scheduler_host_tests/Cargo.toml"));

    assert_eq!(tier.named("ubsan").program, "cargo");
    assert_eq!(tier.named("ubsan").gate, Some("AETHERCORE_RUN_UBSAN"));
    assert!(tier.named("ubsan").args.contains(&"xagent/Cargo.toml"));
}

#[test]
fn sanitizer_assets_exist() {
    fs::file("host_rust_tests/Cargo.toml");
    fs::file("host_tools/scheduler_host_tests/Cargo.toml");
    fs::file("xagent/Cargo.toml");
}
