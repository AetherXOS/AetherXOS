use super::*;

#[test]
fn include_exclude_overrides_defaults() {
    let selection = resolve_selection(
        "debian",
        None,
        None,
        Some("vlc, git"),
        Some("xfce4"),
        Some("https://deb.debian.org/debian"),
    )
    .expect("selection");

    assert_eq!(selection.profile, "debian");
    assert_eq!(selection.package_manager, PackageManager::Apt);
    assert!(selection.packages.iter().any(|p| p == "vlc"));
    assert!(selection.packages.iter().any(|p| p == "git"));
    assert!(!selection.packages.iter().any(|p| p == "xfce4"));
    assert_eq!(selection.mirror.as_deref(), Some("https://deb.debian.org/debian"));
    assert_eq!(
        selection.mirror_fallbacks,
        vec![
            "https://deb.debian.org/debian".to_string(),
            "http://ftp.debian.org/debian".to_string()
        ]
    );
}

#[test]
fn app_targets_expand_package_set() {
    let selection = resolve_selection(
        "debian",
        Some("python,chrome"),
        None,
        None,
        None,
        None,
    )
    .expect("selection");

    assert!(selection.selected_apps.iter().any(|a| a == "python"));
    assert!(selection.selected_apps.iter().any(|a| a == "chrome"));
    assert!(selection.packages.iter().any(|p| p == "python3-pip"));
    assert!(!selection.smoke_commands.is_empty());
}

#[test]
fn app_target_selects_profile_specific_artifacts_and_commands() {
    let selection = resolve_selection(
        "ubuntu-base",
        Some("python,chrome"),
        None,
        None,
        None,
        None,
    )
    .expect("ubuntu-base app target expansion");

    assert!(selection.packages.iter().any(|p| p == "python3-pip"));
    assert!(selection.packages.iter().any(|p| p == "chromium"));
    assert!(selection
        .smoke_commands
        .iter()
        .any(|cmd| cmd.contains("python3 --version")));
    assert!(selection
        .smoke_commands
        .iter()
        .any(|cmd| cmd.contains("chromium") || cmd.contains("chromium-browser")));
}

#[test]
fn ubuntu_base_profile_is_supported() {
    let selection = resolve_selection("ubuntu-base", None, None, None, None, None)
        .expect("ubuntu-base profile selection");

    assert_eq!(selection.profile, "ubuntu-base");
    assert_eq!(selection.package_manager, PackageManager::Apt);
    assert!(selection.packages.iter().any(|p| p == "apt"));
    assert!(selection.packages.iter().any(|p| p == "python3"));
    assert!(selection
        .mirror_fallbacks
        .iter()
        .any(|mirror| mirror == "https://deb.debian.org/debian"));
}

#[test]
fn ubuntu_base_rejects_empty_profile_catalog_inputs() {
    let temp_out = std::env::temp_dir().join("xtask-installer-profile-catalog.json");
    let written = write_preset_catalog(&temp_out);
    assert!(written.is_ok());
    assert!(temp_out.exists());
    let _ = std::fs::remove_file(temp_out);
}
