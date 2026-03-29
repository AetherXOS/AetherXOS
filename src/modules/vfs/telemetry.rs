use core::sync::atomic::{AtomicU64, Ordering};

static FATFS_BRIDGE_PROBES: AtomicU64 = AtomicU64::new(0);
static VFS_INVALID_PATH_REJECTS: AtomicU64 = AtomicU64::new(0);
static RAMFS_OPEN_CALLS: AtomicU64 = AtomicU64::new(0);
static RAMFS_CREATE_CALLS: AtomicU64 = AtomicU64::new(0);
static RAMFS_REMOVE_CALLS: AtomicU64 = AtomicU64::new(0);
static DISK_READ_CALLS: AtomicU64 = AtomicU64::new(0);
static DISK_READ_TOTAL_TICKS: AtomicU64 = AtomicU64::new(0);
static DISK_READ_MAX_TICKS: AtomicU64 = AtomicU64::new(0);
static DISK_WRITE_CALLS: AtomicU64 = AtomicU64::new(0);
static DISK_WRITE_TOTAL_TICKS: AtomicU64 = AtomicU64::new(0);
static DISK_WRITE_MAX_TICKS: AtomicU64 = AtomicU64::new(0);
static DISK_READ_LAT_BUCKET_0: AtomicU64 = AtomicU64::new(0);
static DISK_READ_LAT_BUCKET_1: AtomicU64 = AtomicU64::new(0);
static DISK_READ_LAT_BUCKET_2: AtomicU64 = AtomicU64::new(0);
static DISK_READ_LAT_BUCKET_3: AtomicU64 = AtomicU64::new(0);
static DISK_READ_LAT_BUCKET_4: AtomicU64 = AtomicU64::new(0);
static DISK_WRITE_LAT_BUCKET_0: AtomicU64 = AtomicU64::new(0);
static DISK_WRITE_LAT_BUCKET_1: AtomicU64 = AtomicU64::new(0);
static DISK_WRITE_LAT_BUCKET_2: AtomicU64 = AtomicU64::new(0);
static DISK_WRITE_LAT_BUCKET_3: AtomicU64 = AtomicU64::new(0);
static DISK_WRITE_LAT_BUCKET_4: AtomicU64 = AtomicU64::new(0);

const DISK_LAT_BUCKET_UPPER_BOUNDS: [u64; 5] = [0, 2, 7, 31, u64::MAX];

#[derive(Debug, Clone, Copy)]
pub struct VfsBridgeStats {
    pub fatfs_bridge_probes: u64,
    pub invalid_path_rejects: u64,
    pub ramfs_open_calls: u64,
    pub ramfs_create_calls: u64,
    pub ramfs_remove_calls: u64,
    pub disk_read_calls: u64,
    pub disk_read_avg_ticks: u64,
    pub disk_read_latency_p50_ticks: u64,
    pub disk_read_latency_p95_ticks: u64,
    pub disk_read_latency_p99_ticks: u64,
    pub disk_read_latency_max_ticks: u64,
    pub disk_write_calls: u64,
    pub disk_write_avg_ticks: u64,
    pub disk_write_latency_p50_ticks: u64,
    pub disk_write_latency_p95_ticks: u64,
    pub disk_write_latency_p99_ticks: u64,
    pub disk_write_latency_max_ticks: u64,
    pub inode_cache_hits: u64,
    pub inode_cache_misses: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct VfsDiskIoLatencyStats {
    pub read_calls: u64,
    pub read_avg_ticks: u64,
    pub read_p50_ticks: u64,
    pub read_p95_ticks: u64,
    pub read_p99_ticks: u64,
    pub read_max_ticks: u64,
    pub write_calls: u64,
    pub write_avg_ticks: u64,
    pub write_p50_ticks: u64,
    pub write_p95_ticks: u64,
    pub write_p99_ticks: u64,
    pub write_max_ticks: u64,
}

#[inline(always)]
fn latency_bucket_idx(delta_ticks: u64) -> usize {
    if delta_ticks == 0 {
        return 0;
    }
    if delta_ticks <= 2 {
        return 1;
    }
    if delta_ticks <= 7 {
        return 2;
    }
    if delta_ticks <= 31 {
        return 3;
    }
    4
}

fn histogram_percentile_ticks(buckets: [u64; 5], total_samples: u64, percentile: u64) -> u64 {
    if total_samples == 0 {
        return 0;
    }
    let rank = ((total_samples - 1).saturating_mul(percentile)) / 100;
    let mut cumulative = 0u64;
    for (idx, count) in buckets.iter().enumerate() {
        cumulative = cumulative.saturating_add(*count);
        if cumulative > rank {
            return DISK_LAT_BUCKET_UPPER_BOUNDS[idx];
        }
    }
    DISK_LAT_BUCKET_UPPER_BOUNDS[4]
}

pub fn disk_io_latency_stats() -> VfsDiskIoLatencyStats {
    if !(crate::generated_consts::TELEMETRY_ENABLED
        && crate::generated_consts::TELEMETRY_ENABLE_VFS)
    {
        return VfsDiskIoLatencyStats {
            read_calls: 0,
            read_avg_ticks: 0,
            read_p50_ticks: 0,
            read_p95_ticks: 0,
            read_p99_ticks: 0,
            read_max_ticks: 0,
            write_calls: 0,
            write_avg_ticks: 0,
            write_p50_ticks: 0,
            write_p95_ticks: 0,
            write_p99_ticks: 0,
            write_max_ticks: 0,
        };
    }

    let read_calls = DISK_READ_CALLS.load(Ordering::Relaxed);
    let read_total = DISK_READ_TOTAL_TICKS.load(Ordering::Relaxed);
    let read_max = DISK_READ_MAX_TICKS.load(Ordering::Relaxed);
    let read_buckets = [
        DISK_READ_LAT_BUCKET_0.load(Ordering::Relaxed),
        DISK_READ_LAT_BUCKET_1.load(Ordering::Relaxed),
        DISK_READ_LAT_BUCKET_2.load(Ordering::Relaxed),
        DISK_READ_LAT_BUCKET_3.load(Ordering::Relaxed),
        DISK_READ_LAT_BUCKET_4.load(Ordering::Relaxed),
    ];

    let write_calls = DISK_WRITE_CALLS.load(Ordering::Relaxed);
    let write_total = DISK_WRITE_TOTAL_TICKS.load(Ordering::Relaxed);
    let write_max = DISK_WRITE_MAX_TICKS.load(Ordering::Relaxed);
    let write_buckets = [
        DISK_WRITE_LAT_BUCKET_0.load(Ordering::Relaxed),
        DISK_WRITE_LAT_BUCKET_1.load(Ordering::Relaxed),
        DISK_WRITE_LAT_BUCKET_2.load(Ordering::Relaxed),
        DISK_WRITE_LAT_BUCKET_3.load(Ordering::Relaxed),
        DISK_WRITE_LAT_BUCKET_4.load(Ordering::Relaxed),
    ];

    VfsDiskIoLatencyStats {
        read_calls,
        read_avg_ticks: if read_calls == 0 {
            0
        } else {
            read_total / read_calls
        },
        read_p50_ticks: histogram_percentile_ticks(read_buckets, read_calls, 50),
        read_p95_ticks: histogram_percentile_ticks(read_buckets, read_calls, 95),
        read_p99_ticks: histogram_percentile_ticks(read_buckets, read_calls, 99),
        read_max_ticks: read_max,
        write_calls,
        write_avg_ticks: if write_calls == 0 {
            0
        } else {
            write_total / write_calls
        },
        write_p50_ticks: histogram_percentile_ticks(write_buckets, write_calls, 50),
        write_p95_ticks: histogram_percentile_ticks(write_buckets, write_calls, 95),
        write_p99_ticks: histogram_percentile_ticks(write_buckets, write_calls, 99),
        write_max_ticks: write_max,
    }
}

pub fn bridge_stats() -> VfsBridgeStats {
    if !(crate::generated_consts::TELEMETRY_ENABLED
        && crate::generated_consts::TELEMETRY_ENABLE_VFS)
    {
        return VfsBridgeStats {
            fatfs_bridge_probes: 0,
            invalid_path_rejects: 0,
            ramfs_open_calls: 0,
            ramfs_create_calls: 0,
            ramfs_remove_calls: 0,
            disk_read_calls: 0,
            disk_read_avg_ticks: 0,
            disk_read_latency_p50_ticks: 0,
            disk_read_latency_p95_ticks: 0,
            disk_read_latency_p99_ticks: 0,
            disk_read_latency_max_ticks: 0,
            disk_write_calls: 0,
            disk_write_avg_ticks: 0,
            disk_write_latency_p50_ticks: 0,
            disk_write_latency_p95_ticks: 0,
            disk_write_latency_p99_ticks: 0,
            disk_write_latency_max_ticks: 0,
            inode_cache_hits: 0,
            inode_cache_misses: 0,
        };
    }

    let lat = disk_io_latency_stats();
    let (hits, misses) = crate::modules::vfs::cache::GLOBAL_INODE_CACHE.stats();
    VfsBridgeStats {
        fatfs_bridge_probes: FATFS_BRIDGE_PROBES.load(Ordering::Relaxed),
        invalid_path_rejects: VFS_INVALID_PATH_REJECTS.load(Ordering::Relaxed),
        ramfs_open_calls: RAMFS_OPEN_CALLS.load(Ordering::Relaxed),
        ramfs_create_calls: RAMFS_CREATE_CALLS.load(Ordering::Relaxed),
        ramfs_remove_calls: RAMFS_REMOVE_CALLS.load(Ordering::Relaxed),
        disk_read_calls: lat.read_calls,
        disk_read_avg_ticks: lat.read_avg_ticks,
        disk_read_latency_p50_ticks: lat.read_p50_ticks,
        disk_read_latency_p95_ticks: lat.read_p95_ticks,
        disk_read_latency_p99_ticks: lat.read_p99_ticks,
        disk_read_latency_max_ticks: lat.read_max_ticks,
        disk_write_calls: lat.write_calls,
        disk_write_avg_ticks: lat.write_avg_ticks,
        disk_write_latency_p50_ticks: lat.write_p50_ticks,
        disk_write_latency_p95_ticks: lat.write_p95_ticks,
        disk_write_latency_p99_ticks: lat.write_p99_ticks,
        disk_write_latency_max_ticks: lat.write_max_ticks,
        inode_cache_hits: hits as u64,
        inode_cache_misses: misses as u64,
    }
}

#[inline(always)]
pub(crate) fn note_invalid_path() {
    if !(crate::generated_consts::TELEMETRY_ENABLED
        && crate::generated_consts::TELEMETRY_ENABLE_VFS)
    {
        return;
    }
    VFS_INVALID_PATH_REJECTS.fetch_add(1, Ordering::Relaxed);
}

#[inline(always)]
pub(crate) fn note_ramfs_open() {
    if !(crate::generated_consts::TELEMETRY_ENABLED
        && crate::generated_consts::TELEMETRY_ENABLE_VFS)
    {
        return;
    }
    RAMFS_OPEN_CALLS.fetch_add(1, Ordering::Relaxed);
}

#[inline(always)]
pub(crate) fn note_ramfs_create() {
    if !(crate::generated_consts::TELEMETRY_ENABLED
        && crate::generated_consts::TELEMETRY_ENABLE_VFS)
    {
        return;
    }
    RAMFS_CREATE_CALLS.fetch_add(1, Ordering::Relaxed);
}

#[inline(always)]
pub(crate) fn note_ramfs_remove() {
    if !(crate::generated_consts::TELEMETRY_ENABLED
        && crate::generated_consts::TELEMETRY_ENABLE_VFS)
    {
        return;
    }
    RAMFS_REMOVE_CALLS.fetch_add(1, Ordering::Relaxed);
}

#[inline(always)]
fn update_max(target: &AtomicU64, value: u64) {
    let mut current = target.load(Ordering::Relaxed);
    while value > current {
        match target.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

#[inline(always)]
pub(crate) fn note_disk_read_latency(delta_ticks: u64) {
    if !(crate::generated_consts::TELEMETRY_ENABLED
        && crate::generated_consts::TELEMETRY_ENABLE_VFS)
    {
        return;
    }

    DISK_READ_CALLS.fetch_add(1, Ordering::Relaxed);
    DISK_READ_TOTAL_TICKS.fetch_add(delta_ticks, Ordering::Relaxed);
    update_max(&DISK_READ_MAX_TICKS, delta_ticks);

    match latency_bucket_idx(delta_ticks) {
        0 => {
            DISK_READ_LAT_BUCKET_0.fetch_add(1, Ordering::Relaxed);
        }
        1 => {
            DISK_READ_LAT_BUCKET_1.fetch_add(1, Ordering::Relaxed);
        }
        2 => {
            DISK_READ_LAT_BUCKET_2.fetch_add(1, Ordering::Relaxed);
        }
        3 => {
            DISK_READ_LAT_BUCKET_3.fetch_add(1, Ordering::Relaxed);
        }
        _ => {
            DISK_READ_LAT_BUCKET_4.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[inline(always)]
pub(crate) fn note_disk_write_latency(delta_ticks: u64) {
    if !(crate::generated_consts::TELEMETRY_ENABLED
        && crate::generated_consts::TELEMETRY_ENABLE_VFS)
    {
        return;
    }

    DISK_WRITE_CALLS.fetch_add(1, Ordering::Relaxed);
    DISK_WRITE_TOTAL_TICKS.fetch_add(delta_ticks, Ordering::Relaxed);
    update_max(&DISK_WRITE_MAX_TICKS, delta_ticks);

    match latency_bucket_idx(delta_ticks) {
        0 => {
            DISK_WRITE_LAT_BUCKET_0.fetch_add(1, Ordering::Relaxed);
        }
        1 => {
            DISK_WRITE_LAT_BUCKET_1.fetch_add(1, Ordering::Relaxed);
        }
        2 => {
            DISK_WRITE_LAT_BUCKET_2.fetch_add(1, Ordering::Relaxed);
        }
        3 => {
            DISK_WRITE_LAT_BUCKET_3.fetch_add(1, Ordering::Relaxed);
        }
        _ => {
            DISK_WRITE_LAT_BUCKET_4.fetch_add(1, Ordering::Relaxed);
        }
    }
}

#[cfg(feature = "vfs_fatfs")]
pub fn probe_fatfs_bridge() -> bool {
    if crate::generated_consts::TELEMETRY_ENABLED && crate::generated_consts::TELEMETRY_ENABLE_VFS {
        FATFS_BRIDGE_PROBES.fetch_add(1, Ordering::Relaxed);
    }
    let _opts = crate::modules::vfs::library_backends::FatFsLibrary::new();
    true
}

#[cfg(not(feature = "vfs_fatfs"))]
pub fn probe_fatfs_bridge() -> bool {
    if crate::generated_consts::TELEMETRY_ENABLED && crate::generated_consts::TELEMETRY_ENABLE_VFS {
        FATFS_BRIDGE_PROBES.fetch_add(1, Ordering::Relaxed);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn disk_latency_stats_percentiles_are_monotonic() {
        note_disk_read_latency(0);
        note_disk_read_latency(2);
        note_disk_read_latency(8);
        note_disk_write_latency(1);
        note_disk_write_latency(7);
        note_disk_write_latency(32);

        let stats = disk_io_latency_stats();
        assert!(stats.read_p50_ticks <= stats.read_p95_ticks);
        assert!(stats.read_p95_ticks <= stats.read_p99_ticks);
        assert!(stats.write_p50_ticks <= stats.write_p95_ticks);
        assert!(stats.write_p95_ticks <= stats.write_p99_ticks);
    }
}
