use super::*;

#[test]
fn integration_gated_host_tests_pin_host_target() {
    let specs = integration_specs(false, "x86_64-unknown-linux-gnu");

    let kasan = specs.iter().find(|spec| spec.label == "kasan").unwrap();
    let kmsan = specs.iter().find(|spec| spec.label == "kmsan").unwrap();
    let ubsan = specs.iter().find(|spec| spec.label == "ubsan").unwrap();

    assert_eq!(kasan.program, "cargo");
    assert_eq!(kmsan.program, "cargo");
    assert_eq!(ubsan.program, "cargo");
    assert!(
        kasan
            .args
            .windows(2)
            .any(|pair| pair == ["--target", "x86_64-unknown-linux-gnu"])
    );
    assert!(
        kmsan
            .args
            .windows(2)
            .any(|pair| pair == ["--target", "x86_64-unknown-linux-gnu"])
    );
    assert!(
        ubsan
            .args
            .windows(2)
            .any(|pair| pair == ["--target", "x86_64-unknown-linux-gnu"])
    );
}

#[test]
fn nextest_ci_profile_is_opt_in_and_host_scoped() {
    let dev = nextest_spec("fast", false, "aarch64-unknown-linux-gnu");
    let ci = nextest_spec("fast", true, "aarch64-unknown-linux-gnu");

    assert!(!dev.args.windows(2).any(|pair| pair == ["--profile", "ci"]));
    assert!(ci.args.windows(2).any(|pair| pair == ["--profile", "ci"]));
    assert!(
        ci.args
            .windows(2)
            .any(|pair| pair == ["--target", "aarch64-unknown-linux-gnu"])
    );
}

#[test]
fn cargo_subcommand_probe_names_match_tooling_contracts() {
    assert_eq!(label_for_probe("geiger"), "geiger");
    assert_eq!(label_for_probe("kani"), "kani");
    assert_eq!(label_for_probe("cargofuzz"), "fuzz");
}

#[test]
fn run_all_accepts_core_tiers_in_order() {
    let tiers = ["fast", "integration", "nightly"];
    let host = "x86_64-unknown-linux-gnu";

    let labels: Vec<_> = tiers
        .into_iter()
        .flat_map(|tier| tier_specs(tier, false, host).unwrap())
        .map(|spec| spec.label)
        .collect();

    assert_eq!(labels.first().copied(), Some("nextest"));
    assert!(labels.len() >= 3);
    assert_eq!(tier_specs("fast", false, host).unwrap()[0].label, "nextest");
    assert_eq!(
        tier_specs("integration", false, host).unwrap()[0].label,
        "nextest"
    );
    assert_eq!(
        tier_specs("nightly", false, host).unwrap()[0].label,
        "nextest"
    );
}

#[test]
fn tier_specs_rejects_unknown_phase_names() {
    let err = tier_specs("p0", false, "x86_64-unknown-linux-gnu").unwrap_err();
    let text = format!("{err:#}");
    assert!(text.contains("unknown test phase"));
}
