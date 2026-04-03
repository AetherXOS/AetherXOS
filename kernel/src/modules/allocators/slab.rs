use crate::kernel::sync::IrqSafeMutex;
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

mod alloc_path;
#[path = "slab/allocator_impl.rs"]
mod allocator_impl;
#[path = "slab/segments.rs"]
mod segments;
use segments::{SCache, SlabSegment};

const BLOCK_SIZES: &[usize] = &[32, 64, 128, 256, 512, 1024, 2048, 4096];
const NUM_SIZES: usize = 8;
const MAX_CPUS: usize = crate::generated_consts::KERNEL_MAX_CPUS;
const MAX_TRACKED_SEGMENTS: usize = crate::generated_consts::MEM_SLAB_MAX_TRACKED_SEGMENTS;
const RECLAIM_PROFILE_UNSET: usize = 0;
const RECLAIM_PROFILE_CONSERVATIVE: usize = 1;
const RECLAIM_PROFILE_BALANCED: usize = 2;
const RECLAIM_PROFILE_AGGRESSIVE: usize = 3;
const TELEMETRY_BUCKET_0: u64 = 0;
const TELEMETRY_BUCKET_1: u64 = 1;
const TELEMETRY_BUCKET_2_3_MAX: u64 = 3;
const TELEMETRY_BUCKET_4_7_MAX: u64 = 7;
const TELEMETRY_BUCKET_GE8: u64 = 8;

static SLAB_REFILL_BYTES: AtomicUsize =
    AtomicUsize::new(crate::generated_consts::MEM_SLAB_REFILL_BYTES);
static SLAB_CACHE_LIMIT: AtomicUsize =
    AtomicUsize::new(crate::generated_consts::MEM_SLAB_CACHE_LIMIT);
static SLAB_RELEASE_BATCH: AtomicUsize =
    AtomicUsize::new(crate::generated_consts::MEM_SLAB_RELEASE_BATCH);
static SLAB_ENABLE_CROSS_CPU_STEAL: AtomicBool =
    AtomicBool::new(crate::generated_consts::MEM_SLAB_CROSS_CPU_STEAL);
static SLAB_PRESSURE_SCAN_BUDGET: AtomicUsize =
    AtomicUsize::new(crate::generated_consts::MEM_SLAB_PRESSURE_SCAN_BUDGET);
static SLAB_RECLAIM_PROFILE_MODE: AtomicUsize = AtomicUsize::new(RECLAIM_PROFILE_UNSET);

static SLAB_ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static SLAB_ALLOC_FAST_HITS: AtomicU64 = AtomicU64::new(0);
static SLAB_ALLOC_REFILL_HITS: AtomicU64 = AtomicU64::new(0);
static SLAB_ALLOC_STEAL_HITS: AtomicU64 = AtomicU64::new(0);
static SLAB_ALLOC_FALLBACK_HITS: AtomicU64 = AtomicU64::new(0);
static SLAB_REFILL_FAILURES: AtomicU64 = AtomicU64::new(0);
static SLAB_SEGMENT_TRACK_FAILURES: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_SUCCESSES: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_SWEEPED_BLOCKS: AtomicU64 = AtomicU64::new(0);
static SLAB_PRESSURE_RECLAIM_PASSES: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_LATENCY_TOTAL_TICKS: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_LATENCY_MAX_TICKS: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_LAT_BUCKET_0: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_LAT_BUCKET_1: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_LAT_BUCKET_2_3: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_LAT_BUCKET_4_7: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_LAT_BUCKET_GE8: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_QUEUE_DEPTH_LAST: AtomicUsize = AtomicUsize::new(0);
static SLAB_RECLAIM_QUEUE_DEPTH_MAX: AtomicUsize = AtomicUsize::new(0);
static SLAB_RECLAIM_QUEUE_DEPTH_SAMPLES: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_QUEUE_DEPTH_TOTAL: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_QD_BUCKET_0: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_QD_BUCKET_1: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_QD_BUCKET_2_3: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_QD_BUCKET_4_7: AtomicU64 = AtomicU64::new(0);
static SLAB_RECLAIM_QD_BUCKET_GE8: AtomicU64 = AtomicU64::new(0);
static SLAB_ACTIVE_SEGMENTS: AtomicUsize = AtomicUsize::new(0);
static SLAB_PEAK_ACTIVE_SEGMENTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
pub struct SlabRuntimeStats {
    pub alloc_calls: u64,
    pub alloc_fast_hits: u64,
    pub alloc_refill_hits: u64,
    pub alloc_steal_hits: u64,
    pub alloc_fallback_hits: u64,
    pub refill_failures: u64,
    pub segment_track_failures: u64,
    pub reclaim_attempts: u64,
    pub reclaim_successes: u64,
    pub reclaim_sweeped_blocks: u64,
    pub pressure_reclaim_passes: u64,
    pub reclaim_latency_avg_ticks: u64,
    pub reclaim_latency_p95_ticks: u64,
    pub reclaim_latency_p99_ticks: u64,
    pub reclaim_latency_max_ticks: u64,
    pub reclaim_queue_depth_last: usize,
    pub reclaim_queue_depth_avg: u64,
    pub reclaim_queue_depth_p95: u64,
    pub reclaim_queue_depth_p99: u64,
    pub reclaim_queue_depth_max: usize,
    pub active_segments: usize,
    pub peak_active_segments: usize,
    pub reclaim_profile: &'static str,
    pub pressure_scan_budget: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabReclaimProfile {
    Conservative,
    Balanced,
    Aggressive,
}

#[derive(Debug, Clone, Copy)]
pub struct SlabTuning {
    pub refill_bytes: usize,
    pub cache_limit: usize,
    pub release_batch: usize,
    pub cross_cpu_steal: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct SlabRuntimeConfig {
    pub tuning: SlabTuning,
    pub reclaim_profile: SlabReclaimProfile,
    pub pressure_scan_budget: usize,
}

impl SlabTuning {
    pub const fn new(
        refill_bytes: usize,
        cache_limit: usize,
        release_batch: usize,
        cross_cpu_steal: bool,
    ) -> Self {
        Self {
            refill_bytes,
            cache_limit,
            release_batch,
            cross_cpu_steal,
        }
    }

    fn normalized(self) -> Self {
        Self {
            refill_bytes: self.refill_bytes.max(4096),
            cache_limit: self.cache_limit.max(1),
            release_batch: self.release_batch.max(1),
            cross_cpu_steal: self.cross_cpu_steal,
        }
    }
}

impl SlabReclaimProfile {
    fn as_mode(self) -> usize {
        match self {
            SlabReclaimProfile::Conservative => RECLAIM_PROFILE_CONSERVATIVE,
            SlabReclaimProfile::Balanced => RECLAIM_PROFILE_BALANCED,
            SlabReclaimProfile::Aggressive => RECLAIM_PROFILE_AGGRESSIVE,
        }
    }

    fn from_mode(mode: usize) -> Self {
        match mode {
            RECLAIM_PROFILE_CONSERVATIVE => SlabReclaimProfile::Conservative,
            RECLAIM_PROFILE_AGGRESSIVE => SlabReclaimProfile::Aggressive,
            _ => SlabReclaimProfile::Balanced,
        }
    }
}

fn default_reclaim_profile_mode() -> usize {
    match crate::generated_consts::MEM_SLAB_RECLAIM_PROFILE {
        "Conservative" | "conservative" => RECLAIM_PROFILE_CONSERVATIVE,
        "Aggressive" | "aggressive" => RECLAIM_PROFILE_AGGRESSIVE,
        _ => RECLAIM_PROFILE_BALANCED,
    }
}

fn reclaim_profile_mode() -> usize {
    let mode = SLAB_RECLAIM_PROFILE_MODE.load(Ordering::Relaxed);
    if mode != RECLAIM_PROFILE_UNSET {
        return mode;
    }
    let default_mode = default_reclaim_profile_mode();
    let _ = SLAB_RECLAIM_PROFILE_MODE.compare_exchange(
        RECLAIM_PROFILE_UNSET,
        default_mode,
        Ordering::Relaxed,
        Ordering::Relaxed,
    );
    let resolved = SLAB_RECLAIM_PROFILE_MODE.load(Ordering::Relaxed);
    if resolved == RECLAIM_PROFILE_UNSET {
        default_mode
    } else {
        resolved
    }
}

fn reclaim_profile_name(mode: usize) -> &'static str {
    match mode {
        RECLAIM_PROFILE_CONSERVATIVE => "Conservative",
        RECLAIM_PROFILE_AGGRESSIVE => "Aggressive",
        _ => "Balanced",
    }
}

#[inline(always)]
fn current_latency_tick() -> u64 {
    crate::kernel::watchdog::global_tick()
}

#[inline(always)]
fn update_max_u64(target: &AtomicU64, value: u64) {
    let mut current = target.load(Ordering::Relaxed);
    while value > current {
        match target.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

#[inline(always)]
fn update_max_usize(target: &AtomicUsize, value: usize) {
    let mut current = target.load(Ordering::Relaxed);
    while value > current {
        match target.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

#[inline(always)]
fn bucket_idx(value: u64) -> usize {
    match value {
        0 => 0,
        1 => 1,
        2..=3 => 2,
        4..=7 => 3,
        _ => 4,
    }
}

#[inline(always)]
fn bucket_upper_bound(idx: usize) -> u64 {
    match idx {
        0 => TELEMETRY_BUCKET_0,
        1 => TELEMETRY_BUCKET_1,
        2 => TELEMETRY_BUCKET_2_3_MAX,
        3 => TELEMETRY_BUCKET_4_7_MAX,
        _ => TELEMETRY_BUCKET_GE8,
    }
}

fn histogram_percentile_from_buckets(
    total_samples: u64,
    buckets: [u64; 5],
    percentile: u64,
) -> u64 {
    if total_samples == 0 {
        return 0;
    }
    let rank = ((total_samples - 1).saturating_mul(percentile)) / 100;
    let mut cumulative = 0u64;
    for (idx, count) in buckets.iter().enumerate() {
        cumulative = cumulative.saturating_add(*count);
        if cumulative > rank {
            return bucket_upper_bound(idx);
        }
    }
    TELEMETRY_BUCKET_GE8
}

#[inline(always)]
fn record_telemetry_bucket(delta: u64, buckets: [&AtomicU64; 5]) {
    match bucket_idx(delta) {
        0 => buckets[0].fetch_add(1, Ordering::Relaxed),
        1 => buckets[1].fetch_add(1, Ordering::Relaxed),
        2 => buckets[2].fetch_add(1, Ordering::Relaxed),
        3 => buckets[3].fetch_add(1, Ordering::Relaxed),
        _ => buckets[4].fetch_add(1, Ordering::Relaxed),
    };
}

pub fn slab_tuning() -> SlabTuning {
    SlabTuning {
        refill_bytes: SLAB_REFILL_BYTES.load(Ordering::Relaxed),
        cache_limit: SLAB_CACHE_LIMIT.load(Ordering::Relaxed),
        release_batch: SLAB_RELEASE_BATCH.load(Ordering::Relaxed),
        cross_cpu_steal: SLAB_ENABLE_CROSS_CPU_STEAL.load(Ordering::Relaxed),
    }
}

pub fn set_slab_tuning(tuning: SlabTuning) {
    let normalized = tuning.normalized();
    SLAB_REFILL_BYTES.store(normalized.refill_bytes, Ordering::Relaxed);
    SLAB_CACHE_LIMIT.store(normalized.cache_limit, Ordering::Relaxed);
    SLAB_RELEASE_BATCH.store(normalized.release_batch, Ordering::Relaxed);
    SLAB_ENABLE_CROSS_CPU_STEAL.store(normalized.cross_cpu_steal, Ordering::Relaxed);
}

pub fn runtime_stats() -> SlabRuntimeStats {
    let profile_mode = reclaim_profile_mode();
    let reclaim_attempts = SLAB_RECLAIM_ATTEMPTS.load(Ordering::Relaxed);
    let reclaim_latency_total = SLAB_RECLAIM_LATENCY_TOTAL_TICKS.load(Ordering::Relaxed);
    let reclaim_latency_buckets = [
        SLAB_RECLAIM_LAT_BUCKET_0.load(Ordering::Relaxed),
        SLAB_RECLAIM_LAT_BUCKET_1.load(Ordering::Relaxed),
        SLAB_RECLAIM_LAT_BUCKET_2_3.load(Ordering::Relaxed),
        SLAB_RECLAIM_LAT_BUCKET_4_7.load(Ordering::Relaxed),
        SLAB_RECLAIM_LAT_BUCKET_GE8.load(Ordering::Relaxed),
    ];
    let reclaim_qd_samples = SLAB_RECLAIM_QUEUE_DEPTH_SAMPLES.load(Ordering::Relaxed);
    let reclaim_qd_total = SLAB_RECLAIM_QUEUE_DEPTH_TOTAL.load(Ordering::Relaxed);
    let reclaim_qd_buckets = [
        SLAB_RECLAIM_QD_BUCKET_0.load(Ordering::Relaxed),
        SLAB_RECLAIM_QD_BUCKET_1.load(Ordering::Relaxed),
        SLAB_RECLAIM_QD_BUCKET_2_3.load(Ordering::Relaxed),
        SLAB_RECLAIM_QD_BUCKET_4_7.load(Ordering::Relaxed),
        SLAB_RECLAIM_QD_BUCKET_GE8.load(Ordering::Relaxed),
    ];
    SlabRuntimeStats {
        alloc_calls: SLAB_ALLOC_CALLS.load(Ordering::Relaxed),
        alloc_fast_hits: SLAB_ALLOC_FAST_HITS.load(Ordering::Relaxed),
        alloc_refill_hits: SLAB_ALLOC_REFILL_HITS.load(Ordering::Relaxed),
        alloc_steal_hits: SLAB_ALLOC_STEAL_HITS.load(Ordering::Relaxed),
        alloc_fallback_hits: SLAB_ALLOC_FALLBACK_HITS.load(Ordering::Relaxed),
        refill_failures: SLAB_REFILL_FAILURES.load(Ordering::Relaxed),
        segment_track_failures: SLAB_SEGMENT_TRACK_FAILURES.load(Ordering::Relaxed),
        reclaim_attempts,
        reclaim_successes: SLAB_RECLAIM_SUCCESSES.load(Ordering::Relaxed),
        reclaim_sweeped_blocks: SLAB_RECLAIM_SWEEPED_BLOCKS.load(Ordering::Relaxed),
        pressure_reclaim_passes: SLAB_PRESSURE_RECLAIM_PASSES.load(Ordering::Relaxed),
        reclaim_latency_avg_ticks: if reclaim_attempts == 0 {
            0
        } else {
            reclaim_latency_total / reclaim_attempts
        },
        reclaim_latency_p95_ticks: histogram_percentile_from_buckets(
            reclaim_attempts,
            reclaim_latency_buckets,
            95,
        ),
        reclaim_latency_p99_ticks: histogram_percentile_from_buckets(
            reclaim_attempts,
            reclaim_latency_buckets,
            99,
        ),
        reclaim_latency_max_ticks: SLAB_RECLAIM_LATENCY_MAX_TICKS.load(Ordering::Relaxed),
        reclaim_queue_depth_last: SLAB_RECLAIM_QUEUE_DEPTH_LAST.load(Ordering::Relaxed),
        reclaim_queue_depth_avg: if reclaim_qd_samples == 0 {
            0
        } else {
            reclaim_qd_total / reclaim_qd_samples
        },
        reclaim_queue_depth_p95: histogram_percentile_from_buckets(
            reclaim_qd_samples,
            reclaim_qd_buckets,
            95,
        ),
        reclaim_queue_depth_p99: histogram_percentile_from_buckets(
            reclaim_qd_samples,
            reclaim_qd_buckets,
            99,
        ),
        reclaim_queue_depth_max: SLAB_RECLAIM_QUEUE_DEPTH_MAX.load(Ordering::Relaxed),
        active_segments: SLAB_ACTIVE_SEGMENTS.load(Ordering::Relaxed),
        peak_active_segments: SLAB_PEAK_ACTIVE_SEGMENTS.load(Ordering::Relaxed),
        reclaim_profile: reclaim_profile_name(profile_mode),
        pressure_scan_budget: SLAB_PRESSURE_SCAN_BUDGET.load(Ordering::Relaxed),
    }
}

pub fn slab_reclaim_profile() -> SlabReclaimProfile {
    SlabReclaimProfile::from_mode(reclaim_profile_mode())
}

pub fn set_slab_reclaim_profile(profile: SlabReclaimProfile) {
    SLAB_RECLAIM_PROFILE_MODE.store(profile.as_mode(), Ordering::Relaxed);
}

pub fn slab_pressure_scan_budget() -> usize {
    SLAB_PRESSURE_SCAN_BUDGET
        .load(Ordering::Relaxed)
        .clamp(1, 256)
}

pub fn set_slab_pressure_scan_budget(value: usize) {
    SLAB_PRESSURE_SCAN_BUDGET.store(value.clamp(1, 256), Ordering::Relaxed);
}

pub fn slab_runtime_config() -> SlabRuntimeConfig {
    SlabRuntimeConfig {
        tuning: slab_tuning(),
        reclaim_profile: slab_reclaim_profile(),
        pressure_scan_budget: slab_pressure_scan_budget(),
    }
}

pub fn set_slab_runtime_config(config: SlabRuntimeConfig) {
    set_slab_tuning(config.tuning);
    set_slab_reclaim_profile(config.reclaim_profile);
    set_slab_pressure_scan_budget(config.pressure_scan_budget);
}

#[inline(always)]
fn class_index_for_layout(layout: Layout) -> Option<usize> {
    let size = layout.size();
    let align = layout.align();
    for (i, &s) in BLOCK_SIZES.iter().enumerate() {
        if size <= s && align <= s {
            return Some(i);
        }
    }
    None
}

#[inline(always)]
fn refill_layout_for_block_size(block_size: usize) -> Option<Layout> {
    let refill_bytes = SLAB_REFILL_BYTES.load(Ordering::Relaxed).max(block_size);
    let refill_align = core::cmp::max(block_size, 4096);
    Layout::from_size_align(refill_bytes, refill_align)
        .ok()
        .or_else(|| Layout::from_size_align(block_size, block_size).ok())
}

#[inline(always)]
fn is_block_aligned(ptr: *mut u8, block_size: usize) -> bool {
    (ptr as usize) % block_size == 0
}

pub struct SlabAllocator {
    caches: [[IrqSafeMutex<SCache>; NUM_SIZES]; MAX_CPUS],
    segments: IrqSafeMutex<[SlabSegment; MAX_TRACKED_SEGMENTS]>,
    fallback_allocator: super::linked_list_allocator::LinkedListAllocator,
}

unsafe impl Sync for SlabAllocator {}

macro_rules! repeat_8 {
    ($e:expr) => {
        [$e, $e, $e, $e, $e, $e, $e, $e]
    };
}

macro_rules! repeat_64 {
    ($e:expr) => {
        [
            $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e,
            $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e, $e,
        ]
    };
}

impl SlabAllocator {
    pub const fn new() -> Self {
        Self {
            caches: repeat_64!(repeat_8!(IrqSafeMutex::new(SCache::new()))),
            segments: IrqSafeMutex::new([SlabSegment::EMPTY; MAX_TRACKED_SEGMENTS]),
            fallback_allocator: super::linked_list_allocator::LinkedListAllocator::new(),
        }
    }
}

use crate::interfaces::memory::HeapAllocator;

impl HeapAllocator for SlabAllocator {
    fn init(&self, start: usize, size: usize) {
        #[cfg(target_arch = "x86_64")]
        crate::hal::serial::write_raw("[EARLY SERIAL] slab init begin\n");
        self.fallback_allocator.init(start, size);
        #[cfg(target_arch = "x86_64")]
        crate::hal::serial::write_raw("[EARLY SERIAL] slab init returned\n");
    }
}

unsafe impl GlobalAlloc for SlabAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        SLAB_ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);

        if let Some(idx) = class_index_for_layout(layout) {
            // Safety: the allocator implementation enforces class/layout invariants internally.
            return unsafe { self.alloc_small_class(idx, layout) };
        }

        SLAB_ALLOC_FALLBACK_HITS.fetch_add(1, Ordering::Relaxed);
        // Safety: forwarded directly from the `GlobalAlloc` contract.
        unsafe { self.fallback_allocator.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if ptr.is_null() {
            return;
        }

        if let Some(idx) = class_index_for_layout(layout) {
            // Safety: `ptr` was allocated by this allocator for the same `layout`.
            unsafe { self.dealloc_small_class(ptr, layout, idx) };
            return;
        }

        // Safety: forwarded directly from the `GlobalAlloc` contract.
        unsafe { self.fallback_allocator.dealloc(ptr, layout) };
    }
}

#[cfg(test)]
mod tests;
