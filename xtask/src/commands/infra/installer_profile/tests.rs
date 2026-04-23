use super::*;

fn mock_preset_catalog() -> InstallerPresetCatalog {
    InstallerPresetCatalog {
        presets: vec![
            InstallerPreset {
                id: "debian".to_string(),
                title: "Debian".to_string(),
                description: "Debian".to_string(),
                package_manager: PackageManager::Apt,
                default_packages: vec!["apt".to_string()],
                mirror_fallbacks: vec![
                    "https://deb.debian.org/debian".to_string(),
                    "http://ftp.debian.org/debian".to_string(),
                ],
            },
            InstallerPreset {
                id: "ubuntu-base".to_string(),
                title: "Ubuntu Base".to_string(),
                description: "Ubuntu Base".to_string(),
                package_manager: PackageManager::Apt,
                default_packages: vec!["apt".to_string(), "python3".to_string()],
                mirror_fallbacks: vec!["https://deb.debian.org/debian".to_string()],
            },
        ],
    }
}

fn mock_app_target_catalog() -> InstallerAppTargetCatalog {
    let mut packages_by_profile = BTreeMap::new();
    packages_by_profile.insert("*".to_string(), vec!["python3-pip".to_string()]);

    let mut python_target = InstallerAppTarget {
        id: "python".to_string(),
        title: "Python".to_string(),
        description: "Python".to_string(),
        packages_by_profile,
        download_artifacts_by_profile: BTreeMap::new(),
        smoke_commands_by_profile: BTreeMap::new(),
    };

    let mut packages_by_profile_chrome = BTreeMap::new();
    packages_by_profile_chrome.insert("*".to_string(), vec!["chrome".to_string()]);
    packages_by_profile_chrome.insert("ubuntu-base".to_string(), vec!["chromium".to_string()]);

    let mut smoke_commands_chrome = BTreeMap::new();
    smoke_commands_chrome.insert("*".to_string(), vec!["chrome --version".to_string()]);
    smoke_commands_chrome.insert(
        "ubuntu-base".to_string(),
        vec!["chromium --version".to_string()],
    );

    let chrome_target = InstallerAppTarget {
        id: "chrome".to_string(),
        title: "Chrome".to_string(),
        description: "Chrome".to_string(),
        packages_by_profile: packages_by_profile_chrome,
        download_artifacts_by_profile: BTreeMap::new(),
        smoke_commands_by_profile: smoke_commands_chrome,
    };

    let mut smoke_commands_python = BTreeMap::new();
    smoke_commands_python.insert("*".to_string(), vec!["python3 --version".to_string()]);
    python_target.smoke_commands_by_profile = smoke_commands_python;

    InstallerAppTargetCatalog {
        targets: vec![python_target, chrome_target],
    }
}

#[test]
fn include_exclude_overrides_defaults() {
    let selection = resolve_selection_from_catalogs(
        &mock_preset_catalog(),
        &mock_app_target_catalog(),
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
    assert_eq!(
        selection.mirror.as_deref(),
        Some("https://deb.debian.org/debian")
    );
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
    let selection = resolve_selection_from_catalogs(
        &mock_preset_catalog(),
        &mock_app_target_catalog(),
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
    let selection = resolve_selection_from_catalogs(
        &mock_preset_catalog(),
        &mock_app_target_catalog(),
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
    assert!(
        selection
            .smoke_commands
            .iter()
            .any(|cmd| cmd.contains("python3 --version"))
    );
    assert!(
        selection
            .smoke_commands
            .iter()
            .any(|cmd| cmd.contains("chromium") || cmd.contains("chromium-browser"))
    );
}

#[test]
fn ubuntu_base_profile_is_supported() {
    let selection = resolve_selection_from_catalogs(
        &mock_preset_catalog(),
        &mock_app_target_catalog(),
        "ubuntu-base",
        None,
        None,
        None,
        None,
        None,
    )
    .expect("ubuntu-base profile selection");

    assert_eq!(selection.profile, "ubuntu-base");
    assert_eq!(selection.package_manager, PackageManager::Apt);
    assert!(selection.packages.iter().any(|p| p == "apt"));
    assert!(selection.packages.iter().any(|p| p == "python3"));
    assert!(
        selection
            .mirror_fallbacks
            .iter()
            .any(|mirror| mirror == "https://deb.debian.org/debian")
    );
}

#[test]
fn ubuntu_base_rejects_empty_profile_catalog_inputs() {
    let catalog = InstallerPresetCatalog { presets: vec![] };
    assert!(validate_catalog(&catalog).is_err());
}
