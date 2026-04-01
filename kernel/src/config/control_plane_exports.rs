use super::*;
#[cfg(feature = "vfs")]
use super::support::{
    build_feature_report, build_feature_summary_report, build_linux_compat_readiness_report,
    build_runtime_keys_report, build_snapshot_report, normalize_export_dir,
};

impl KernelConfig {
    #[cfg(feature = "vfs")]
    pub fn export_snapshot_to_mount(
        mount_path: &str,
        export_dir: &str,
    ) -> Result<usize, &'static str> {
        let mount_id = crate::kernel::vfs_control::mount_id_by_path(mount_path.as_bytes())
            .ok_or("mount path not found")?;
        let tid = crate::interfaces::task::TaskId(0);
        let normalized_dir = normalize_export_dir(export_dir);
        let _ = crate::kernel::vfs_control::ramfs_mkdir(mount_id, normalized_dir.as_str(), tid);

        let snapshot = Self::snapshot();
        let features = Self::feature_controls();
        let summaries = Self::feature_category_summaries();
        let drift = Self::feature_runtime_drift_count();
        let readiness = Self::linux_compat_readiness();
        let blockers = Self::linux_compat_blocker_details();
        let next_action = Self::linux_compat_next_action();
        let keys = Self::runtime_override_template();
        let audit = Self::audit_stats();

        let files = [
            ("snapshot.txt", build_snapshot_report(&snapshot, &audit)),
            ("features.txt", build_feature_report(&features)),
            (
                "feature_summary.txt",
                build_feature_summary_report(&summaries, drift),
            ),
            (
                "linux_compat_readiness.txt",
                build_linux_compat_readiness_report(&readiness, blockers.as_slice(), next_action),
            ),
            ("runtime_keys.txt", build_runtime_keys_report(&keys)),
        ];

        let mut exported = 0usize;
        for (name, contents) in files {
            let path = alloc::format!("{}/{}", normalized_dir, name);
            let mut file =
                match crate::kernel::vfs_control::ramfs_create_file(mount_id, path.as_str(), tid) {
                    Ok(file) => file,
                    Err(_) => {
                        crate::kernel::vfs_control::ramfs_open_file(mount_id, path.as_str(), tid)?
                    }
                };
            let _ = file.seek(crate::modules::vfs::SeekFrom::Start(0));
            let wrote = file.write(contents.as_bytes())?;
            file.flush()?;
            if wrote != contents.len() {
                return Err("short config export write");
            }
            exported += 1;
        }

        #[cfg(feature = "linux_compat")]
        {
            exported += crate::modules::linux_compat::export_compat_surfaces_to_mount(
                mount_path,
                &alloc::format!("{}/compat", normalized_dir),
            )?;
        }

        Ok(exported)
    }

    #[cfg(all(feature = "vfs", feature = "linux_compat"))]
    pub fn mount_compat_config_surface(mount_path: &str) -> Result<usize, &'static str> {
        crate::modules::linux_compat::mount_compat_surface_fs(mount_path)
    }

    #[cfg(all(feature = "vfs", feature = "linux_compat"))]
    pub fn refresh_compat_config_surface(mount_path: &str) -> Result<usize, &'static str> {
        crate::modules::linux_compat::refresh_compat_surface_fs(mount_path)
    }
}
