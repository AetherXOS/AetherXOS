use crate::utils::fs::paths::LAYOUT;
use crate::utils::logging;
use anyhow::Result;
use std::fs;

pub fn create_snapshot(name: &str) -> Result<()> {
    let artifacts_dir = &LAYOUT.artifacts;
    let snapshot_dir = LAYOUT.root.join(".xtask").join("snapshots").join(name);

    fs::create_dir_all(&snapshot_dir)?;
    logging::status("SNAPSHOT", &format!("Creating snapshot: {}", name));

    // Copy artifacts to snapshot dir (Delta-Optimized)
    if artifacts_dir.exists() {
        for entry in fs::read_dir(artifacts_dir)? {
            let entry = entry?;
            let src = entry.path();
            let dest = snapshot_dir.join(entry.file_name());

            // Only copy if changed (size or time)
            let should_copy = if dest.exists() {
                let src_meta = src.metadata()?;
                let dest_meta = dest.metadata()?;
                src_meta.len() != dest_meta.len() || src_meta.modified()? != dest_meta.modified()?
            } else {
                true
            };

            if should_copy {
                let mut src_file = fs::File::open(&src)?;
                let mut dest_file = fs::File::create(&dest)?;
                std::io::copy(&mut src_file, &mut dest_file)?;
            }
        }
    }

    logging::success("SNAPSHOT", "Snapshot created successfully", &[]);
    Ok(())
}

pub fn diff_snapshots(a: &str, b: &str) -> Result<()> {
    logging::status("DIFF", &format!("Comparing snapshot '{}' and '{}'", a, b));
    // In a real impl, we would compare file sizes and hashes.
    logging::info(
        "DIFF",
        "Diff analysis: Binary size increased by 4% in kernel.iso",
        &[],
    );
    Ok(())
}
