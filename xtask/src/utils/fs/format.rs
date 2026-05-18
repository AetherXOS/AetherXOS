/// Utility module for formatting file sizes into human-readable strings.

/// Formats a size in bytes into a human-readable string (B, KB, MB, GB, TB, PB, EB).
/// Scales automatically and uses precise decimal formatting for readability.
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else if size >= 100.0 {
        format!("{:.0} {}", size, UNITS[unit_idx])
    } else if size >= 10.0 {
        format!("{:.1} {}", size, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}
