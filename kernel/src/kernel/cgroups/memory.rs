/// Cgroup Memory Controller — per-group memory limits and tracking.
///
/// Implements `memory.max`, `memory.high`, `memory.low` semantics
/// matching cgroups v2 behaviour.
/// Memory controller state for one cgroup.
#[derive(Debug, Clone)]
pub struct MemoryController {
    /// Hard limit (bytes). 0 = unlimited. Triggers OOM if exceeded.
    pub max: u64,
    /// High watermark (bytes). Triggers reclaim pressure above this.
    pub high: u64,
    /// Low protection (bytes). Memory below this is reclaim-protected.
    pub low: u64,
    /// Minimum guarantee (bytes). Cannot be reclaimed below this.
    pub min: u64,
    /// Swap limit (bytes). 0 = no swap allowed for this cgroup.
    pub swap_max: u64,
    /// Current RSS usage (bytes).
    pub usage: u64,
    /// Current swap usage (bytes).
    pub swap_usage: u64,
    /// Peak RSS usage ever observed.
    pub peak: u64,
    /// Number of OOM kills triggered by this cgroup.
    pub oom_kills: u64,
    /// Whether OOM killer is enabled for this cgroup (vs pause).
    pub oom_group: bool,
}

impl MemoryController {
    pub fn new() -> Self {
        Self {
            max: 0,
            high: 0,
            low: 0,
            min: 0,
            swap_max: 0,
            usage: 0,
            swap_usage: 0,
            peak: 0,
            oom_kills: 0,
            oom_group: true,
        }
    }

    /// Charge `bytes` to this cgroup. Returns false if it would exceed `max`.
    pub fn try_charge(&mut self, bytes: u64) -> bool {
        let effective_max = current_effective_memory_limit(self.max);
        if effective_max > 0 && self.usage + bytes > effective_max {
            return false;
        }
        self.usage += bytes;
        if self.usage > self.peak {
            self.peak = self.usage;
        }
        true
    }

    /// Uncharge `bytes` from this cgroup.
    pub fn uncharge(&mut self, bytes: u64) {
        self.usage = self.usage.saturating_sub(bytes);
    }

    /// Charge swap usage.
    pub fn try_charge_swap(&mut self, bytes: u64) -> bool {
        let effective_swap_max = current_effective_memory_limit(self.swap_max);
        if effective_swap_max > 0 && self.swap_usage + bytes > effective_swap_max {
            return false;
        }
        self.swap_usage += bytes;
        true
    }

    /// Uncharge swap usage.
    pub fn uncharge_swap(&mut self, bytes: u64) {
        self.swap_usage = self.swap_usage.saturating_sub(bytes);
    }

    /// Returns true if the cgroup is under memory pressure (above high watermark).
    pub fn is_under_pressure(&self) -> bool {
        let effective_high = current_effective_memory_limit(self.high);
        effective_high > 0 && self.usage > effective_high
    }

    /// Returns true if the cgroup's usage is within the reclaim-protected low range.
    pub fn is_protected(&self) -> bool {
        let effective_low = current_effective_memory_protection(self.low);
        effective_low > 0 && self.usage <= effective_low
    }

    /// Record an OOM kill event.
    pub fn record_oom_kill(&mut self) {
        self.oom_kills += 1;
    }

    /// Reset peak usage counter.
    pub fn reset_peak(&mut self) {
        self.peak = self.usage;
    }
}

#[inline(always)]
fn governor_adjusted_memory_budget(budget: u64, latency_bias: &'static str) -> u64 {
    crate::kernel::virt_bias::adjust_budget_u64(budget, latency_bias)
}

#[inline(always)]
fn governor_adjusted_memory_protection(budget: u64, latency_bias: &'static str) -> u64 {
    crate::kernel::virt_bias::adjust_inverse_budget_u64(budget, latency_bias)
}

#[inline(always)]
fn current_effective_memory_limit(budget: u64) -> u64 {
    governor_adjusted_memory_budget(budget, crate::kernel::virt_bias::current_latency_bias())
}

#[inline(always)]
fn current_effective_memory_protection(budget: u64) -> u64 {
    governor_adjusted_memory_protection(budget, crate::kernel::virt_bias::current_latency_bias())
}

#[cfg(test)]
#[path = "memory/tests.rs"]
mod tests;
