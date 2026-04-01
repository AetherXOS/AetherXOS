/// Cgroup CPU Controller — bandwidth limiting and weight-based sharing.
///
/// Implements `cpu.max` (quota/period) and `cpu.weight` matching
/// cgroups v2 semantics.  Integrates with CFS group scheduling.
/// CPU controller state for one cgroup.
#[derive(Debug, Clone)]
pub struct CpuController {
    /// Weight for proportional sharing (1–10000, default 100).
    pub weight: u32,
    /// Bandwidth quota in microseconds per period. 0 = unlimited.
    pub quota_us: u64,
    /// Period length in microseconds (default 100_000 = 100ms).
    pub period_us: u64,
    /// CPU time consumed in the current period (microseconds).
    pub used_us: u64,
    /// Whether this cgroup is currently throttled.
    pub throttled: bool,
    /// Number of times this cgroup has been throttled.
    pub nr_throttled: u64,
    /// Total throttled time (microseconds).
    pub throttled_time_us: u64,
}

impl CpuController {
    pub fn new() -> Self {
        Self {
            weight: 100,
            quota_us: 0,
            period_us: 100_000,
            used_us: 0,
            throttled: false,
            nr_throttled: 0,
            throttled_time_us: 0,
        }
    }

    /// Set bandwidth limit: `quota_us` per `period_us`.
    pub fn set_max(&mut self, quota_us: u64, period_us: u64) {
        self.quota_us = quota_us;
        self.period_us = if period_us > 0 { period_us } else { 100_000 };
    }

    /// Charge CPU time.  Returns false if throttled.
    pub fn try_charge(&mut self, us: u64) -> bool {
        if self.throttled {
            return false;
        }
        self.used_us += us;
        let effective_quota = current_effective_cpu_quota_us(self.quota_us);
        if effective_quota > 0 && self.used_us >= effective_quota {
            self.throttled = true;
            self.nr_throttled += 1;
            return false;
        }
        true
    }

    /// Reset at the start of a new period.
    pub fn reset_period(&mut self) {
        if self.throttled {
            self.throttled_time_us += self.period_us;
        }
        self.used_us = 0;
        self.throttled = false;
    }

    /// Check if throttled.
    pub fn is_throttled(&self) -> bool {
        self.throttled
    }

    /// Remaining quota in current period.
    pub fn remaining_us(&self) -> u64 {
        if self.quota_us == 0 {
            return u64::MAX;
        }
        current_effective_cpu_quota_us(self.quota_us).saturating_sub(self.used_us)
    }
}

#[inline(always)]
fn governor_adjusted_quota_us(quota_us: u64, latency_bias: &'static str) -> u64 {
    crate::kernel::virt_bias::adjust_budget_u64(quota_us, latency_bias)
}

#[inline(always)]
fn current_effective_cpu_quota_us(quota_us: u64) -> u64 {
    governor_adjusted_quota_us(quota_us, crate::kernel::virt_bias::current_latency_bias())
}

#[cfg(test)]
#[path = "cpu/tests.rs"]
mod tests;
