use std::path::{Path, PathBuf};

/// Automatically discovers workspace binaries and target metadata dynamically.
/// Completely eliminates hardcoded architecture targets or build profile assumptions!
pub struct WorkspacePaths;

impl WorkspacePaths {
    /// Recursively discovers the target directory for any built kernel ELF files.
    /// Safely handles any architecture target (e.g. x86_64, aarch64, riscv64).
    pub fn find_kernel_elf() -> Option<PathBuf> {
        let target_dir = Path::new("target");
        if !target_dir.exists() {
            return None;
        }

        let mut candidate_elfs = Vec::new();
        for entry in walkdir::WalkDir::new(target_dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let name = entry.file_name().to_string_lossy();
                // Find file named exactly 'aethercore' or 'aethercore.elf'
                if name == "aethercore" || name == "aethercore.elf" {
                    let path = entry.path().to_path_buf();
                    // Avoid anything inside incremental directories or compiler internals
                    let path_str = path.to_string_lossy();
                    if !path_str.contains("incremental") && !path_str.contains("deps") && !path_str.contains("build") {
                        if let Ok(metadata) = std::fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                candidate_elfs.push((path, modified));
                            }
                        }
                    }
                }
            }
        }

        // Sort candidates by modification time descending to pick the most recently built binary
        candidate_elfs.sort_by(|a, b| b.1.cmp(&a.1));
        
        candidate_elfs.first().map(|(path, _)| path.clone())
    }

    /// Dynamically determines the active cross-compilation target name.
    pub fn discover_target_arch() -> String {
        if let Some(elf_path) = Self::find_kernel_elf() {
            // Extracts target name from "target/<target-name>/debug/aethercore"
            let components: Vec<_> = elf_path.components().collect();
            if components.len() >= 4 {
                let target_idx = components.len() - 3;
                let target_name = components[target_idx].as_os_str().to_string_lossy().into_owned();
                if target_name != "debug" && target_name != "release" {
                    return target_name;
                }
            }
        }
        "x86_64-unknown-none".to_string() // Stable fallback
    }

    /// Dynamically determines the active build profile (debug or release).
    pub fn discover_build_profile() -> String {
        if let Some(elf_path) = Self::find_kernel_elf() {
            let components: Vec<_> = elf_path.components().collect();
            if components.len() >= 3 {
                let profile = components[components.len() - 2].as_os_str().to_string_lossy().into_owned();
                return profile;
            }
        }
        "debug".to_string() // Stable fallback
    }

    /// Recursively discovers a compiled bootloader image (ISO or IMG) inside the workspace.
    pub fn find_boot_image() -> Option<PathBuf> {
        let mut candidates = Vec::new();
        
        let paths = [
            "artifacts/boot_image/aethercore.iso",
            "artifacts/boot_image/aethercore.img",
            "artifacts/boot_image/stage/boot/aethercore.elf"
        ];
        
        for path in &paths {
            let p = Path::new(path);
            if p.exists() {
                return Some(p.to_path_buf());
            }
        }

        for entry in walkdir::WalkDir::new("artifacts").into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let ext = entry.path().extension().map(|s| s.to_string_lossy()).unwrap_or_default();
                if ext == "iso" || ext == "img" {
                    candidates.push(entry.path().to_path_buf());
                }
            }
        }

        candidates.first().cloned()
    }
}
