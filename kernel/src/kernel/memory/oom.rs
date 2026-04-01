use crate::hal::common::virt::{
    current_virtualization_runtime_governor, GOVERNOR_BIAS_AGGRESSIVE, GOVERNOR_BIAS_RELAXED,
};
/// Out-of-Memory (OOM) killer — selects and terminates the best candidate task
/// when memory pressure becomes critical.
///
/// The OOM score is computed as:
///   `rss_pages * priority_weight + oom_score_adj`
/// where higher scores make a task more likely to be killed.
/// Kernel-essential tasks (pid 0, init) are never killed.
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Threshold (fraction of total) below which OOM is triggered.
/// e.g. 5 means free pages < total_pages / 20  (5%).
const OOM_THRESHOLD_DIVISOR: usize = 20;

/// Global OOM state.
static OOM_ACTIVE: AtomicBool = AtomicBool::new(false);
static OOM_KILLS: AtomicU64 = AtomicU64::new(0);

/// Per-task information the OOM killer needs. Callers fill this in from the
/// process/task registry; the OOM module itself doesn't depend on those structs.
#[derive(Debug, Clone)]
pub struct OomCandidate {
    pub task_id: usize,
    pub process_id: usize,
    /// Name for logging.
    pub name: [u8; 16],
    /// Resident set size in pages.
    pub rss_pages: usize,
    /// User-controlled adjustment (like Linux oom_score_adj, -1000..1000).
    pub oom_score_adj: i32,
    /// True for kernel threads / init — cannot be killed.
    pub unkillable: bool,
}

/// Result of an OOM scan.
#[derive(Debug, Clone, Copy)]
pub enum OomAction {
    /// No kill needed; pressure relieved or no candidates.
    None,
    /// Kill this task (returns task_id).
    Kill(usize),
}

/// Compute OOM score for a candidate.
fn oom_score(c: &OomCandidate) -> i64 {
    if c.unkillable {
        return i64::MIN;
    }
    // Base score proportional to memory usage.
    let base = c.rss_pages as i64;
    // Adjust by user hint.
    let adjusted = base + c.oom_score_adj as i64;
    adjusted
}

/// Select the best OOM victim from candidates.
pub fn select_victim(candidates: &[OomCandidate]) -> OomAction {
    let mut best_score = i64::MIN;
    let mut best_id = None;
    for c in candidates {
        let score = oom_score(c);
        if score > best_score {
            best_score = score;
            best_id = Some(c.task_id);
        }
    }
    match best_id {
        Some(id) if best_score > i64::MIN => {
            OOM_KILLS.fetch_add(1, Ordering::Relaxed);
            OomAction::Kill(id)
        }
        _ => OomAction::None,
    }
}

/// Check whether OOM condition is active based on free/total page counts.
pub fn check_pressure(free_pages: usize, total_pages: usize) -> bool {
    let threshold = total_pages / OOM_THRESHOLD_DIVISOR;
    let active = free_pages < threshold;
    OOM_ACTIVE.store(active, Ordering::Relaxed);
    active
}

/// Returns true if OOM is currently active.
pub fn is_oom_active() -> bool {
    OOM_ACTIVE.load(Ordering::Relaxed)
}

/// Total kills performed.
pub fn kill_count() -> u64 {
    OOM_KILLS.load(Ordering::Relaxed)
}

/// Memory pressure levels for graduated response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureLevel {
    /// Normal operation.
    None,
    /// Low pressure — start background reclaim.
    Low,
    /// Medium pressure — aggressive reclaim, defer allocations.
    Medium,
    /// Critical — OOM kill imminent.
    Critical,
}

/// Evaluate pressure level from free/total page counts.
pub fn pressure_level(free_pages: usize, total_pages: usize) -> PressureLevel {
    if total_pages == 0 {
        return PressureLevel::Critical;
    }
    let pct = (free_pages * 100) / total_pages;
    let (critical_max, medium_max, low_max) =
        pressure_level_thresholds(current_virtualization_runtime_governor().latency_bias);
    if pct <= critical_max {
        PressureLevel::Critical
    } else if pct <= medium_max {
        PressureLevel::Medium
    } else if pct <= low_max {
        PressureLevel::Low
    } else {
        PressureLevel::None
    }
}

#[inline(always)]
fn pressure_level_thresholds(latency_bias: &'static str) -> (usize, usize, usize) {
    match latency_bias {
        GOVERNOR_BIAS_AGGRESSIVE => (6, 12, 24),
        GOVERNOR_BIAS_RELAXED => (3, 7, 15),
        _ => (4, 9, 19),
    }
}

#[cfg(test)]
#[path = "oom/tests.rs"]
mod tests;
