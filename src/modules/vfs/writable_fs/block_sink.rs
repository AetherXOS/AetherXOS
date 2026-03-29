use super::*;

/// Writeback sink that persists pages to a block device.
/// Maintains a simple block allocation bitmap and inode->block mapping.
pub struct BlockWritebackSink {
    /// Block device interface (raw sector I/O).
    /// Uses interior mutability since writeback can happen from any context.
    pub(crate) device: Mutex<Box<dyn BlockDeviceAdapter>>,
    /// Inode->block map: (ino, logical_block) -> physical_block.
    block_map: Mutex<BTreeMap<(u64, u64), u64>>,
    /// Simple bitmap allocator: tracks which blocks are free.
    alloc_bitmap: Mutex<Vec<bool>>,
    /// Next block to scan for allocation.
    next_free: AtomicU64,
    /// Total blocks on device.
    #[allow(dead_code)]
    total_blocks: u64,
    /// Journal area: reserved blocks at the start of the device.
    journal_blocks: u64,
    /// Data area starts after journal.
    data_start_block: u64,
}

/// Adapter trait for block devices (abstracts over concrete driver types).
pub trait BlockDeviceAdapter: Send + Sync {
    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<(), &'static str>;
    fn write_block(&mut self, block: u64, data: &[u8]) -> Result<(), &'static str>;
    fn flush(&mut self) -> Result<(), &'static str>;
    fn block_count(&self) -> u64;
}

impl BlockWritebackSink {
    /// Create a new sink backed by a block device.
    /// Reserves the first `journal_blocks` for the journal area.
    pub fn new(device: Box<dyn BlockDeviceAdapter>, journal_blocks: u64) -> Self {
        let total = device.block_count();
        let data_start = journal_blocks;
        let data_blocks = total.saturating_sub(data_start);
        let bitmap = alloc::vec![false; data_blocks as usize];

        Self {
            device: Mutex::new(device),
            block_map: Mutex::new(BTreeMap::new()),
            alloc_bitmap: Mutex::new(bitmap),
            next_free: AtomicU64::new(0),
            total_blocks: total,
            journal_blocks,
            data_start_block: data_start,
        }
    }

    /// Allocate a physical block.
    fn alloc_block(&self) -> Option<u64> {
        let mut bitmap = self.alloc_bitmap.lock();
        let len = bitmap.len();
        if len == 0 {
            return None;
        }
        let start = self.next_free.load(Ordering::Relaxed) as usize % len;

        // Linear scan from next_free
        for i in 0..len {
            let idx = (start + i) % len;
            if !bitmap[idx] {
                bitmap[idx] = true;
                let next = ((idx + 1) % len) as u64;
                self.next_free.store(next, Ordering::Relaxed);
                return Some(self.data_start_block + idx as u64);
            }
        }
        None
    }

    /// Free a physical block.
    #[allow(dead_code)]
    fn free_block(&self, phys_block: u64) {
        if phys_block >= self.data_start_block {
            let idx = (phys_block - self.data_start_block) as usize;
            let mut bitmap = self.alloc_bitmap.lock();
            if idx < bitmap.len() {
                bitmap[idx] = false;
            }
        }
    }

    /// Get or allocate the physical block for an (ino, logical_block) pair.
    pub(crate) fn ensure_block(&self, ino: u64, logical_block: u64) -> Result<u64, &'static str> {
        let mut map = self.block_map.lock();
        if let Some(&phys) = map.get(&(ino, logical_block)) {
            return Ok(phys);
        }
        let phys = self.alloc_block().ok_or("block device full")?;
        map.insert((ino, logical_block), phys);
        Ok(phys)
    }
}

impl WritebackSink for BlockWritebackSink {
    fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str> {
        let logical_block = offset / PAGE_SIZE as u64;
        let phys_block = self.ensure_block(ino, logical_block)?;
        let mut dev = self.device.lock();
        dev.write_block(phys_block, data)
    }

    fn flush(&self) -> Result<(), &'static str> {
        self.device.lock().flush()
    }

    fn journal_write(&self, entry: &writeback::JournalEntry) -> Result<(), &'static str> {
        // Simple journal: serialize entry to a page-sized buffer and write to journal area.
        // Use entry.seq modulo journal_blocks to determine which journal block to write.
        if self.journal_blocks == 0 {
            return Ok(());
        }
        let journal_slot = entry.seq % self.journal_blocks;
        let mut buf = [0u8; PAGE_SIZE];

        // Simple binary format: [seq:8][op_type:4][payload:variable]
        let seq_bytes = entry.seq.to_le_bytes();
        buf[..8].copy_from_slice(&seq_bytes);

        match &entry.op {
            JournalOp::Commit { txn_id } => {
                buf[8..12].copy_from_slice(&1u32.to_le_bytes());
                buf[12..20].copy_from_slice(&txn_id.to_le_bytes());
            }
            JournalOp::InodeUpdate {
                ino,
                new_size,
                new_mode,
            } => {
                buf[8..12].copy_from_slice(&2u32.to_le_bytes());
                buf[12..20].copy_from_slice(&ino.to_le_bytes());
                buf[20..28].copy_from_slice(&new_size.to_le_bytes());
                buf[28..30].copy_from_slice(&new_mode.to_le_bytes());
            }
            JournalOp::BlockAlloc {
                ino,
                logical_block,
                physical_block,
            } => {
                buf[8..12].copy_from_slice(&3u32.to_le_bytes());
                buf[12..20].copy_from_slice(&ino.to_le_bytes());
                buf[20..28].copy_from_slice(&logical_block.to_le_bytes());
                buf[28..36].copy_from_slice(&physical_block.to_le_bytes());
            }
            JournalOp::BlockFree { physical_block } => {
                buf[8..12].copy_from_slice(&4u32.to_le_bytes());
                buf[12..20].copy_from_slice(&physical_block.to_le_bytes());
            }
            JournalOp::DentryCreate {
                parent_ino,
                name_hash,
                child_ino,
            } => {
                buf[8..12].copy_from_slice(&5u32.to_le_bytes());
                buf[12..20].copy_from_slice(&parent_ino.to_le_bytes());
                buf[20..28].copy_from_slice(&name_hash.to_le_bytes());
                buf[28..36].copy_from_slice(&child_ino.to_le_bytes());
            }
            JournalOp::DentryRemove {
                parent_ino,
                name_hash,
            } => {
                buf[8..12].copy_from_slice(&6u32.to_le_bytes());
                buf[12..20].copy_from_slice(&parent_ino.to_le_bytes());
                buf[20..28].copy_from_slice(&name_hash.to_le_bytes());
            }
        }

        self.device.lock().write_block(journal_slot, &buf)
    }

    fn journal_commit(&self) -> Result<(), &'static str> {
        self.device.lock().flush()
    }
}
