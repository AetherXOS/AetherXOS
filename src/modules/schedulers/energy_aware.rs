use core::sync::atomic::{AtomicU64, Ordering};

static ENERGY_PICK_CALLS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct EnergyAwareStats {
    pub pick_calls: u64,
}

pub fn pick_cpu_by_efficiency(cpu_efficiency: &[u32], runnable_hint: usize) -> Option<usize> {
    ENERGY_PICK_CALLS.fetch_add(1, Ordering::Relaxed);
    if cpu_efficiency.is_empty() {
        return None;
    }

    let mut best_idx = 0usize;
    let mut best_score = 0u32;
    for (idx, eff) in cpu_efficiency.iter().enumerate() {
        let adjusted = eff.saturating_sub((runnable_hint as u32).saturating_div(4));
        if adjusted > best_score {
            best_score = adjusted;
            best_idx = idx;
        }
    }
    Some(best_idx)
}

pub fn energy_aware_stats() -> EnergyAwareStats {
    EnergyAwareStats {
        pick_calls: ENERGY_PICK_CALLS.load(Ordering::Relaxed),
    }
}
