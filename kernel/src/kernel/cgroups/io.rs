/// Cgroup I/O Controller — per-device bandwidth throttling.
///
/// Implements `io.max` (rbps/wbps/riops/wiops per device) matching
/// cgroups v2 semantics.
use alloc::collections::BTreeMap;

/// Per-device I/O limits.
#[derive(Debug, Clone, Copy)]
pub struct IoDeviceLimit {
    /// Read bytes per second (0 = unlimited).
    pub rbps: u64,
    /// Write bytes per second (0 = unlimited).
    pub wbps: u64,
    /// Read I/O operations per second (0 = unlimited).
    pub riops: u64,
    /// Write I/O operations per second (0 = unlimited).
    pub wiops: u64,
}

impl IoDeviceLimit {
    pub fn unlimited() -> Self {
        Self {
            rbps: 0,
            wbps: 0,
            riops: 0,
            wiops: 0,
        }
    }
}

/// Per-device I/O usage tracking.
#[derive(Debug, Clone, Copy, Default)]
pub struct IoDeviceUsage {
    pub rbytes: u64,
    pub wbytes: u64,
    pub rios: u64,
    pub wios: u64,
}

/// I/O controller state for one cgroup.
#[derive(Debug, Clone)]
pub struct IoController {
    /// Per-device (major:minor as u32) limits.
    limits: BTreeMap<u32, IoDeviceLimit>,
    /// Per-device usage counters.
    usage: BTreeMap<u32, IoDeviceUsage>,
    /// Proportional weight (1–10000, default 100).
    pub weight: u32,
}

impl IoController {
    pub fn new() -> Self {
        Self {
            limits: BTreeMap::new(),
            usage: BTreeMap::new(),
            weight: 100,
        }
    }

    /// Set I/O limits for a device.
    pub fn set_device_limit(&mut self, dev_id: u32, limit: IoDeviceLimit) {
        self.limits.insert(dev_id, limit);
    }

    /// Remove I/O limits for a device.
    pub fn remove_device_limit(&mut self, dev_id: u32) {
        self.limits.remove(&dev_id);
    }

    /// Check if a read operation of `bytes` on `dev_id` is allowed.
    pub fn check_read(&self, dev_id: u32, bytes: u64) -> bool {
        if let Some(limit) = self.limits.get(&dev_id) {
            let effective = current_effective_io_limit(*limit);
            if let Some(u) = self.usage.get(&dev_id) {
                if effective.rbps > 0 && u.rbytes + bytes > effective.rbps {
                    return false;
                }
                if effective.riops > 0 && u.rios + 1 > effective.riops {
                    return false;
                }
            }
        }
        true
    }

    /// Check if a write operation of `bytes` on `dev_id` is allowed.
    pub fn check_write(&self, dev_id: u32, bytes: u64) -> bool {
        if let Some(limit) = self.limits.get(&dev_id) {
            let effective = current_effective_io_limit(*limit);
            if let Some(u) = self.usage.get(&dev_id) {
                if effective.wbps > 0 && u.wbytes + bytes > effective.wbps {
                    return false;
                }
                if effective.wiops > 0 && u.wios + 1 > effective.wiops {
                    return false;
                }
            }
        }
        true
    }

    /// Record a read operation.
    pub fn charge_read(&mut self, dev_id: u32, bytes: u64) {
        let u = self.usage.entry(dev_id).or_default();
        u.rbytes += bytes;
        u.rios += 1;
    }

    /// Record a write operation.
    pub fn charge_write(&mut self, dev_id: u32, bytes: u64) {
        let u = self.usage.entry(dev_id).or_default();
        u.wbytes += bytes;
        u.wios += 1;
    }

    /// Reset usage counters (called at start of each accounting period).
    pub fn reset_usage(&mut self) {
        for u in self.usage.values_mut() {
            *u = IoDeviceUsage::default();
        }
    }

    /// Get usage for a device.
    pub fn device_usage(&self, dev_id: u32) -> Option<&IoDeviceUsage> {
        self.usage.get(&dev_id)
    }
}

#[inline(always)]
fn governor_adjusted_io_budget(budget: u64, latency_bias: &'static str) -> u64 {
    crate::kernel::virt_bias::adjust_budget_u64(budget, latency_bias)
}

#[inline(always)]
fn io_limit_with_bias(limit: IoDeviceLimit, latency_bias: &'static str) -> IoDeviceLimit {
    IoDeviceLimit {
        rbps: governor_adjusted_io_budget(limit.rbps, latency_bias),
        wbps: governor_adjusted_io_budget(limit.wbps, latency_bias),
        riops: governor_adjusted_io_budget(limit.riops, latency_bias),
        wiops: governor_adjusted_io_budget(limit.wiops, latency_bias),
    }
}

#[inline(always)]
fn current_effective_io_limit(limit: IoDeviceLimit) -> IoDeviceLimit {
    io_limit_with_bias(limit, crate::kernel::virt_bias::current_latency_bias())
}

#[cfg(test)]
#[path = "io/tests.rs"]
mod tests;
