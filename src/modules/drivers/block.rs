use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum BlockDriverKind {
    Nvme = 1,
    Ahci = 2,
    VirtIoBlock = 3,
}

#[derive(Debug, Clone, Copy)]
pub struct BlockDeviceInfo {
    pub kind: BlockDriverKind,
    pub io_base: u64,
    pub irq: u8,
    pub block_size: u32,
}

pub trait BlockDevice {
    fn info(&self) -> BlockDeviceInfo;
    fn init(&mut self) -> Result<(), &'static str>;
    fn read_blocks(&mut self, lba: u64, count: u16, out: &mut [u8]) -> Result<usize, &'static str>;
    fn write_blocks(&mut self, lba: u64, count: u16, input: &[u8]) -> Result<usize, &'static str>;
    /// Flush volatile write caches to stable storage (e.g. FLUSH CACHE EXT / NVMe Flush).
    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    /// Issue a write barrier — all writes before the barrier are guaranteed to be
    /// on stable storage before any write after the barrier begins.
    fn write_barrier(&mut self) -> Result<(), &'static str> {
        self.flush()
    }
    /// Returns the total number of addressable sectors.
    fn sector_count(&self) -> u64 {
        0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlockDriverStats {
    pub probe_attempts: u64,
    pub probe_success: u64,
    pub init_attempts: u64,
    pub init_success: u64,
    pub io_attempts: u64,
    pub io_success: u64,
    pub io_max_latency_ns: u64,
    pub io_total_latency_ns: u64,
}

static PROBE_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static PROBE_SUCCESS: AtomicU64 = AtomicU64::new(0);
static INIT_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static INIT_SUCCESS: AtomicU64 = AtomicU64::new(0);
static IO_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static IO_SUCCESS: AtomicU64 = AtomicU64::new(0);
static IO_MAX_LATENCY_NS: AtomicU64 = AtomicU64::new(0);
static IO_TOTAL_LATENCY_NS: AtomicU64 = AtomicU64::new(0);

pub fn mark_probe(success: bool) {
    PROBE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    if success {
        PROBE_SUCCESS.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn mark_init(success: bool) {
    INIT_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    if success {
        INIT_SUCCESS.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn mark_io(success: bool, latency_ns: u64) {
    IO_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
    if success {
        IO_SUCCESS.fetch_add(1, Ordering::Relaxed);
    }
    // Simple EWMA or accumulator. For now, max latency track.
    let current_max = IO_MAX_LATENCY_NS.load(Ordering::Relaxed);
    if latency_ns > current_max {
        IO_MAX_LATENCY_NS.store(latency_ns, Ordering::Relaxed);
    }
    IO_TOTAL_LATENCY_NS.fetch_add(latency_ns, Ordering::Relaxed);
}

pub fn stats() -> BlockDriverStats {
    BlockDriverStats {
        probe_attempts: PROBE_ATTEMPTS.load(Ordering::Relaxed),
        probe_success: PROBE_SUCCESS.load(Ordering::Relaxed),
        init_attempts: INIT_ATTEMPTS.load(Ordering::Relaxed),
        init_success: INIT_SUCCESS.load(Ordering::Relaxed),
        io_attempts: IO_ATTEMPTS.load(Ordering::Relaxed),
        io_success: IO_SUCCESS.load(Ordering::Relaxed),
        io_max_latency_ns: IO_MAX_LATENCY_NS.load(Ordering::Relaxed),
        io_total_latency_ns: IO_TOTAL_LATENCY_NS.load(Ordering::Relaxed),
    }
}

pub const SECTOR_SIZE: usize = 512;
