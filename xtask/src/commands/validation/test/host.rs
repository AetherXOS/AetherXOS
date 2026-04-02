use crate::utils::{cargo, logging};
use anyhow::Result;

pub fn validate_feature_matrix(release: bool) -> Result<()> {
    let host_target = cargo::detect_host_triple()?;
    logging::info(
        "test",
        "running host feature matrix",
        &[("target", &host_target), ("release", &release.to_string())],
    );
    let target = Some(host_target.as_str());

    let variants: &[(&str, &str)] = &[
        ("default Rust", ""),
        ("linux_compat feature matrix", "linux_compat,telemetry"),
        ("vfs feature matrix", "vfs,telemetry"),
        ("posix process feature matrix", "posix_process,telemetry"),
        (
            "posix process/signal minimal",
            "posix_process,posix_signal,posix_time,telemetry",
        ),
        ("posix net feature matrix", "posix_net,telemetry"),
        (
            "posix fs/net feature matrix",
            "posix_fs,posix_net,telemetry",
        ),
        ("vfs fs feature matrix", "vfs,posix_fs,telemetry"),
        (
            "posix process/signal combined",
            "vfs,posix_fs,posix_process,posix_signal,posix_time,telemetry",
        ),
        ("posix time feature matrix", "posix_time,telemetry"),
        (
            "integrated posix feature matrix",
            "vfs,posix_fs,posix_net,posix_process,posix_signal,posix_time,telemetry",
        ),
    ];

    for (label, features) in variants {
        cargo::cargo_check_features(label, features, target, release)?;
    }

    logging::ready("test", "host feature matrix passed", &host_target);
    Ok(())
}
