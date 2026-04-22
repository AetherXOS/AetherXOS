use anyhow::Result;

use crate::utils::{paths, report};

use super::models::{Layer, Scorecard};
use super::profile::NormalizedOptions;

pub(super) fn write_reports(
    normalized: NormalizedOptions,
    compat: &Layer,
    desktop_probes: serde_json::Map<String, serde_json::Value>,
    scorecard: &Scorecard,
) -> Result<()> {
    let reports = paths::resolve("reports");
    paths::ensure_dir(&reports)?;
    report::write_json_report(
        &reports.join("linux_app_compat_validation_scorecard.json"),
        scorecard,
    )?;

    let mut runtime_probe = serde_json::Map::new();
    runtime_probe.insert(
        "generated_utc".to_string(),
        serde_json::json!(report::utc_now_iso()),
    );
    runtime_probe.insert(
        "busybox_required".to_string(),
        serde_json::json!(normalized.require_busybox),
    );
    runtime_probe.insert(
        "glibc_required".to_string(),
        serde_json::json!(normalized.require_glibc),
    );
    runtime_probe.insert(
        "desktop_smoke".to_string(),
        serde_json::json!(normalized.desktop_smoke),
    );
    runtime_probe.insert(
        "wayland_required".to_string(),
        serde_json::json!(normalized.require_wayland),
    );
    runtime_probe.insert(
        "x11_required".to_string(),
        serde_json::json!(normalized.require_x11),
    );
    runtime_probe.insert(
        "fs_stack_required".to_string(),
        serde_json::json!(normalized.require_fs_stack),
    );
    runtime_probe.insert(
        "package_stack_required".to_string(),
        serde_json::json!(normalized.require_package_stack),
    );
    runtime_probe.insert(
        "desktop_app_stack_required".to_string(),
        serde_json::json!(normalized.require_desktop_app_stack),
    );
    runtime_probe.insert("layer_counts".to_string(), serde_json::json!(compat));
    runtime_probe.insert(
        "desktop_probes".to_string(),
        serde_json::Value::Object(desktop_probes),
    );

    report::write_json_report(
        &reports.join("linux_app_runtime_probe_report.json"),
        &serde_json::Value::Object(runtime_probe),
    )?;

    Ok(())
}
