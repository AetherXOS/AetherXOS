use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cli::BuildAction;
use crate::commands::infra::installer_policy;
use crate::commands::infra::installer_profile;
use crate::utils::cargo;
use crate::utils::paths;

#[derive(Debug, Serialize)]
struct InstallerSelectionPreview<'a> {
    schema_version: u32,
    profile: &'a str,
    package_manager: &'a str,
    mirror: Option<&'a str>,
    selected_apps: &'a [String],
    packages: &'a [String],
    download_artifacts: &'a [installer_profile::InstallerDownloadArtifact],
    smoke_commands: &'a [String],
    policy: &'a installer_policy::InstallerPolicy,
}

#[derive(Debug, Serialize)]
struct InstallerExecutionPlan<'a> {
    schema_version: u32,
    profile: &'a str,
    stages: Vec<InstallerExecutionStage>,
}

#[derive(Debug, Serialize)]
struct InstallerExecutionStage {
    id: String,
    title: String,
    depends_on: Vec<String>,
    timeout_seconds: u32,
    critical: bool,
}

/// Entry point for `cargo xtask build <action>`.
pub fn execute(action: &BuildAction) -> Result<()> {
    match action {
        BuildAction::Full => full_pipeline(),
        BuildAction::Iso => iso_only(),
        BuildAction::AptIso {
            profile,
            apps,
            packages,
            include,
            exclude,
            mirror,
        } => apt_iso(
            profile,
            apps.as_deref(),
            packages.as_deref(),
            include.as_deref(),
            exclude.as_deref(),
            mirror.as_deref(),
        ),
        BuildAction::Kernel => kernel_only(),
        BuildAction::Initramfs => initramfs_only(),
    }
}

// ---------------------------------------------------------------------------
// Full pipeline: kernel + initramfs + limine config + ISO + smoke
// ---------------------------------------------------------------------------

fn full_pipeline() -> Result<()> {
    println!("[build::full] Starting full OS build pipeline");

    let target = "x86_64-unknown-none";
    let profile = "release";
    let append = "console=ttyS0 loglevel=7";

    // Step 1: Compile kernel
    println!("[build::full] Step 1/5: Compiling kernel (target={}, profile={})", target, profile);
    cargo::cargo(&["build", "--target", target, "--release"])?;

    // Step 2: Locate ELF artifact
    println!("[build::full] Step 2/5: Locating kernel ELF artifact");
    let target_dir = Path::new("target").join(target).join(profile);
    let elf_path = find_elf_artifact(&target_dir)?;
    println!("[build::full]   Found: {}", elf_path.display());

    // Step 3: Stage boot artifacts
    println!("[build::full] Step 3/5: Staging boot artifacts");
    let stage_dir = paths::resolve("artifacts/boot_image/stage/boot");
    paths::ensure_dir(&stage_dir)?;

    let stage_kernel = stage_dir.join("hypercore.elf");
    fs::copy(&elf_path, &stage_kernel).context("Failed to stage kernel ELF")?;

    // Step 4: Generate limine configs
    println!("[build::full] Step 4/5: Generating bootloader configurations");
    crate::commands::infra::limine::generate_configs(
        &stage_dir,
        "hypercore.elf",
        "initramfs.cpio.gz",
        append,
    )?;

    // Step 5: Generate initramfs
    println!("[build::full] Step 5/5: Building initramfs archive");
    let initramfs_dir = paths::resolve("boot/initramfs");
    let initramfs_out = stage_dir.join("initramfs.cpio.gz");
    crate::commands::infra::initramfs::build(&initramfs_dir, &initramfs_out)?;

    println!("[build::full] Pipeline completed successfully.");
    Ok(())
}

fn iso_only() -> Result<()> {
    println!("[build::iso] Building bootable ISO image");
    // Build kernel + stage first, then assemble ISO
    full_pipeline()?;
    let stage_dir = paths::resolve("artifacts/boot_image/stage/boot");
    let iso_out = paths::resolve("artifacts/boot_image/hypercore.iso");
    crate::commands::infra::iso::assemble(&stage_dir, &iso_out)?;
    println!("[build::iso] ISO written: {}", iso_out.display());
    Ok(())
}

fn apt_iso(
    profile: &str,
    apps: Option<&str>,
    packages: Option<&str>,
    include: Option<&str>,
    exclude: Option<&str>,
    mirror: Option<&str>,
) -> Result<()> {
    println!("[build::apt-iso] Building apt-seeded ISO image");

    let selection = installer_profile::resolve_selection(
        profile,
        apps,
        packages,
        include,
        exclude,
        mirror,
    )?;
    let policy = installer_policy::resolve_policy(&selection.profile)?;
    installer_profile::write_preset_catalog(&paths::resolve(
        "artifacts/tooling/installer/presets.json",
    ))?;
    write_selection_preview(&selection, &policy)?;
    write_execution_plan(&selection, &policy)?;

    full_pipeline()?;

    let generated_root = paths::resolve("artifacts/boot_image/generated/initramfs_apt");
    if generated_root.exists() {
        fs::remove_dir_all(&generated_root)
            .with_context(|| format!("failed to clean {}", generated_root.display()))?;
    }

    let initramfs_source = paths::resolve("boot/initramfs");
    copy_dir_recursive(&initramfs_source, &generated_root)?;

    crate::commands::infra::userspace_seed::inject_seed(
        &generated_root,
        &selection,
        &policy,
        &paths::resolve("artifacts/userspace_apps"),
    )?;

    let stage_dir = paths::resolve("artifacts/boot_image/stage/boot");
    let initramfs_out = stage_dir.join("initramfs.cpio.gz");
    crate::commands::infra::initramfs::build(&generated_root, &initramfs_out)?;

    let iso_out = paths::resolve("artifacts/boot_image/hypercore-apt.iso");
    crate::commands::infra::iso::assemble(&stage_dir, &iso_out)?;

    println!("[build::apt-iso] ISO written: {}", iso_out.display());
    println!("[build::apt-iso] Profile: {}", selection.profile);
    if !selection.selected_apps.is_empty() {
        println!(
            "[build::apt-iso] App targets: {}",
            selection.selected_apps.join(",")
        );
    }
    println!(
        "[build::apt-iso] Preset catalog: {}",
        paths::resolve("artifacts/tooling/installer/presets.json").display()
    );
    println!(
        "[build::apt-iso] Seed package list: {}",
        generated_root
            .join("etc/hypercore/apt-preload-packages.txt")
            .display()
    );
    println!(
        "[build::apt-iso] Seed bundles: {}",
        generated_root
            .join("usr/share/hypercore/userspace_apps")
            .display()
    );

    Ok(())
}

fn kernel_only() -> Result<()> {
    println!("[build::kernel] Compiling kernel only");
    cargo::cargo(&["build", "--target", "x86_64-unknown-none", "--release"])?;
    let target_dir = Path::new("target").join("x86_64-unknown-none").join("release");
    let elf = find_elf_artifact(&target_dir)?;
    println!("[build::kernel] Kernel ELF: {}", elf.display());
    Ok(())
}

fn initramfs_only() -> Result<()> {
    println!("[build::initramfs] Generating initramfs archive");
    let initramfs_dir = paths::resolve("boot/initramfs");
    let out = paths::resolve("artifacts/boot_image/stage/boot/initramfs.cpio.gz");
    paths::ensure_dir(out.parent().unwrap())?;
    crate::commands::infra::initramfs::build(&initramfs_dir, &out)?;
    println!("[build::initramfs] Archive written: {}", out.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Scan a directory for the first file with a valid ELF magic header (>1KB).
fn find_elf_artifact(dir: &Path) -> Result<PathBuf> {
    if !dir.exists() {
        bail!("Target directory not found: {}", dir.display());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let meta = entry.metadata()?;
            if meta.len() > 1024 {
                let mut buf = [0u8; 4];
                let file = fs::File::open(&path)?;
                use std::io::Read;
                let mut reader = std::io::BufReader::new(file);
                if reader.read_exact(&mut buf).is_ok() && buf == *b"\x7fELF" {
                    return Ok(path);
                }
            }
        }
    }
    bail!("No ELF artifact found in {}", dir.display())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        bail!("source directory not found: {}", src.display());
    }
    fs::create_dir_all(dst)?;

    for entry in WalkDir::new(src).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let rel = path
            .strip_prefix(src)
            .with_context(|| format!("failed strip_prefix for {}", path.display()))?;
        let target = dst.join(rel);

        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, &target).with_context(|| {
            format!("failed to copy {} -> {}", path.display(), target.display())
        })?;
    }

    Ok(())
}

fn write_selection_preview(
    selection: &installer_profile::InstallerSelection,
    policy: &installer_policy::InstallerPolicy,
) -> Result<()> {
    let out_path = paths::resolve("artifacts/tooling/installer/selection_preview.json");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let package_manager = match selection.package_manager {
        installer_profile::PackageManager::Apt => "apt",
        installer_profile::PackageManager::Pacman => "pacman",
    };
    let preview = InstallerSelectionPreview {
        schema_version: 1,
        profile: &selection.profile,
        package_manager,
        mirror: selection.mirror.as_deref(),
        selected_apps: &selection.selected_apps,
        packages: &selection.packages,
        download_artifacts: &selection.download_artifacts,
        smoke_commands: &selection.smoke_commands,
        policy,
    };

    fs::write(&out_path, serde_json::to_string_pretty(&preview)?)?;
    println!("[build::apt-iso] Selection preview: {}", out_path.display());
    Ok(())
}

fn write_execution_plan(
    selection: &installer_profile::InstallerSelection,
    policy: &installer_policy::InstallerPolicy,
) -> Result<()> {
    let out_path = paths::resolve("artifacts/tooling/installer/execution_plan.json");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut stages = vec![
        InstallerExecutionStage {
            id: "artifact-fetch".to_string(),
            title: "Fetch external artifacts".to_string(),
            depends_on: vec![],
            timeout_seconds: policy.install_timeout_seconds,
            critical: policy.checksum_required,
        },
        InstallerExecutionStage {
            id: "repo-metadata-verify".to_string(),
            title: "Verify repository metadata signatures".to_string(),
            depends_on: vec!["artifact-fetch".to_string()],
            timeout_seconds: policy.install_timeout_seconds,
            critical: policy.metadata_signature_required,
        },
        InstallerExecutionStage {
            id: "package-install".to_string(),
            title: "Install selected packages".to_string(),
            depends_on: vec!["repo-metadata-verify".to_string()],
            timeout_seconds: policy.install_timeout_seconds,
            critical: true,
        },
        InstallerExecutionStage {
            id: "postinstall-hooks".to_string(),
            title: "Run post-install hooks".to_string(),
            depends_on: vec!["package-install".to_string()],
            timeout_seconds: policy.install_timeout_seconds,
            critical: false,
        },
    ];

    if !selection.smoke_commands.is_empty() {
        stages.push(InstallerExecutionStage {
            id: "app-smoke".to_string(),
            title: "Run app target smoke tests".to_string(),
            depends_on: vec!["postinstall-hooks".to_string()],
            timeout_seconds: policy.smoke_timeout_seconds,
            critical: true,
        });
    }

    let plan = InstallerExecutionPlan {
        schema_version: 1,
        profile: &selection.profile,
        stages,
    };

    fs::write(&out_path, serde_json::to_string_pretty(&plan)?)?;
    println!("[build::apt-iso] Execution plan: {}", out_path.display());
    Ok(())
}
