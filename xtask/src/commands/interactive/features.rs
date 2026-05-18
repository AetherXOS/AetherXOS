use crate::commands::interactive::config;
use crate::utils::{core::features as feature_utils, logging};
use anyhow::Result;
// KernelFeatures removed

pub fn manage_features() -> Result<()> {
    logging::status("FEATURES", "Managing Kernel Features");

    let mut current_config = config::get_active_config()?;

    // We can't easily convert back from KernelFeatures bitmask to strings perfectly if some names overlap,
    // but we can just use the strings we saved.

    let selected = feature_utils::prompt_kernel_feature_selection(
        "Interactive Kernel Config",
        &["vfs", "linux_compat"],
    )?;

    // Ideally prompt_kernel_feature_selection would return the names too.
    // For now, let's assume we want to save the result.
    // Since KernelFeatures doesn't easily expose the names it was built from (without extra logic),
    // we'll just log the success.

    logging::success(
        "FEATURES",
        "Kernel features updated for this session",
        &[("selection", &format!("{:?}", selected))],
    );

    // Update config
    current_config.last_features = vec![format!("{:?}", selected)]; // Placeholder until we have name list
    config::save_active_config(&current_config)?;

    Ok(())
}
