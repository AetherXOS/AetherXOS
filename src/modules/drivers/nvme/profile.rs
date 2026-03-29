use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::*;

const NVME_MIN_IO_QUEUE_DEPTH: usize = 2;
const NVME_DEFAULT_IO_QUEUE_DEPTH: usize = 64;
const NVME_MAX_IO_QUEUE_DEPTH: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvmeQueueProfile {
    LowLatency,
    Balanced,
    Throughput,
}

impl NvmeQueueProfile {
    const fn default() -> Self {
        Self::Balanced
    }
}

#[inline(always)]
const fn profile_to_raw(p: NvmeQueueProfile) -> u64 {
    match p {
        NvmeQueueProfile::LowLatency => 0,
        NvmeQueueProfile::Balanced => 1,
        NvmeQueueProfile::Throughput => 2,
    }
}

#[inline(always)]
const fn raw_to_profile(r: u64) -> NvmeQueueProfile {
    match r {
        0 => NvmeQueueProfile::LowLatency,
        2 => NvmeQueueProfile::Throughput,
        _ => NvmeQueueProfile::Balanced,
    }
}

#[inline(always)]
const fn profile_default_depth(p: NvmeQueueProfile) -> usize {
    match p {
        NvmeQueueProfile::LowLatency => 32,
        NvmeQueueProfile::Balanced => NVME_DEFAULT_IO_QUEUE_DEPTH,
        NvmeQueueProfile::Throughput => 256,
    }
}

static NVME_QUEUE_PROFILE_RAW: AtomicU64 =
    AtomicU64::new(profile_to_raw(NvmeQueueProfile::default()));
static NVME_IO_QUEUE_DEPTH_OVERRIDE: AtomicUsize = AtomicUsize::new(0);

pub fn set_nvme_queue_profile(p: NvmeQueueProfile) {
    NVME_QUEUE_PROFILE_RAW.store(profile_to_raw(p), Ordering::Relaxed);
}

pub fn nvme_queue_profile() -> NvmeQueueProfile {
    raw_to_profile(NVME_QUEUE_PROFILE_RAW.load(Ordering::Relaxed))
}

pub fn set_nvme_io_queue_depth_override(depth: Option<usize>) {
    NVME_IO_QUEUE_DEPTH_OVERRIDE.store(depth.unwrap_or(0), Ordering::Relaxed);
}

pub fn nvme_io_queue_depth_override() -> Option<usize> {
    let ov = NVME_IO_QUEUE_DEPTH_OVERRIDE.load(Ordering::Relaxed);
    if ov == 0 {
        None
    } else {
        Some(ov)
    }
}

pub fn nvme_effective_io_queue_depth() -> usize {
    let ov = NVME_IO_QUEUE_DEPTH_OVERRIDE.load(Ordering::Relaxed);
    if ov != 0 {
        return ov.clamp(NVME_MIN_IO_QUEUE_DEPTH, NVME_MAX_IO_QUEUE_DEPTH);
    }
    profile_default_depth(nvme_queue_profile())
        .clamp(NVME_MIN_IO_QUEUE_DEPTH, NVME_MAX_IO_QUEUE_DEPTH)
}

#[derive(Debug, Clone, Copy)]
pub struct NvmeWaitStats {
    pub disable_ready_timeout_spins: usize,
    pub disable_ready_timeouts: u64,
    pub controller_ready_timeout_spins: usize,
    pub controller_ready_timeouts: u64,
    pub admin_timeout_spins: usize,
    pub admin_timeouts: u64,
    pub io_timeout_spins: usize,
    pub io_timeouts: u64,
}

pub fn wait_stats() -> NvmeWaitStats {
    NvmeWaitStats {
        disable_ready_timeout_spins: KernelConfig::nvme_disable_ready_timeout_spins(),
        disable_ready_timeouts: NVME_DISABLE_READY_TIMEOUTS.load(Ordering::Relaxed),
        controller_ready_timeout_spins: KernelConfig::nvme_poll_timeout_spins(),
        controller_ready_timeouts: NVME_CONTROLLER_READY_TIMEOUTS.load(Ordering::Relaxed),
        admin_timeout_spins: KernelConfig::nvme_poll_timeout_spins(),
        admin_timeouts: NVME_ADMIN_TIMEOUTS.load(Ordering::Relaxed),
        io_timeout_spins: KernelConfig::nvme_io_timeout_spins(),
        io_timeouts: NVME_IO_TIMEOUTS.load(Ordering::Relaxed),
    }
}
