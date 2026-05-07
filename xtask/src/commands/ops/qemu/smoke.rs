use serde::Serialize;

pub const PANIC_MARKERS: &[&str] = &[
    "PANIC report:",
    "[KERNEL DUMP] panic_count=",
    "kernel panic",
];

pub const BOOT_SUCCESS_MARKERS: &[&str] = &[
    "limine: Loading executable",
    "smp: Successfully brought up AP",
    "[linux_compat] init complete",
    "[aether_init] early userspace bootstrap",
    "installer-seed-complete",
];

#[derive(Serialize)]
pub struct QemuSmokeSummary {
    pub mode: String,
    pub duration_sec: f64,
    pub timed_out: bool,
    pub panic_seen: bool,
    pub boot_marker_seen: bool,
    pub interrupt_health_ok: bool,
    pub success: bool,
    pub pass: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct InterruptHealth {
    pub total: u64,
    pub timer: u64,
    pub non_timer: u64,
    pub dropped: u64,
    pub dispatch_attempted: u64,
    pub dispatch_handled: u64,
}

pub fn detect_interrupt_health(stream: &str) -> Option<InterruptHealth> {
    if let Some(line) = stream.lines().rev().find(|line| line.contains("x86_64 irq stats:")) {
        let total = extract_u64_kv(line, "total")?;
        let timer = extract_u64_kv(line, "timer")?;
        let non_timer = extract_u64_kv(line, "non_timer")?;
        let dropped = extract_u64_kv(line, "dropped")?;
        let dispatch_attempted = extract_u64_kv(line, "dispatch_attempted")?;
        let dispatch_handled = extract_u64_kv(line, "dispatch_handled")?;
        return Some(InterruptHealth { total, timer, non_timer, dropped, dispatch_attempted, dispatch_handled });
    }
    None
}

pub fn validate_interrupt_health(health: InterruptHealth) -> bool {
    health.total >= health.timer.saturating_add(health.non_timer) &&
    health.dispatch_attempted >= health.dispatch_handled &&
    health.dropped <= health.total
}

fn extract_u64_kv(line: &str, key: &str) -> Option<u64> {
    let needle = format!("{}=", key);
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    rest[..end].trim_end_matches(',').parse::<u64>().ok()
}
