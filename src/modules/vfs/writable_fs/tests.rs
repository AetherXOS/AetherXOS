use super::*;
use alloc::vec;

struct FakeBlockDevice {
    blocks: Vec<Vec<u8>>,
    flushes: u64,
}

impl FakeBlockDevice {
    fn new(block_count: usize) -> Self {
        Self {
            blocks: vec![vec![0u8; PAGE_SIZE]; block_count],
            flushes: 0,
        }
    }
}

impl BlockDeviceAdapter for FakeBlockDevice {
    fn read_block(&mut self, block: u64, buf: &mut [u8]) -> Result<(), &'static str> {
        let data = self
            .blocks
            .get(block as usize)
            .ok_or("block out of range")?;
        if buf.len() != data.len() {
            return Err("buffer size mismatch");
        }
        buf.copy_from_slice(data);
        Ok(())
    }

    fn write_block(&mut self, block: u64, data: &[u8]) -> Result<(), &'static str> {
        let dst = self
            .blocks
            .get_mut(block as usize)
            .ok_or("block out of range")?;
        if dst.len() != data.len() {
            return Err("block size mismatch");
        }
        dst.copy_from_slice(data);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), &'static str> {
        self.flushes += 1;
        Ok(())
    }

    fn block_count(&self) -> u64 {
        self.blocks.len() as u64
    }
}

#[test_case]
fn block_writeback_sink_allocates_distinct_blocks_and_reuses_existing_mapping() {
    let sink = BlockWritebackSink::new(Box::new(FakeBlockDevice::new(8)), 2);

    let first = sink.ensure_block(11, 0).expect("first block");
    let same = sink.ensure_block(11, 0).expect("same logical block");
    let second = sink.ensure_block(11, 1).expect("second logical block");

    assert_eq!(first, same);
    assert_ne!(first, second);
    assert!(first >= 2);
    assert!(second >= 2);
}

#[test_case]
fn block_writeback_sink_journal_write_targets_expected_slot() {
    let sink = BlockWritebackSink::new(Box::new(FakeBlockDevice::new(6)), 2);
    let entry = writeback::JournalEntry {
        seq: 3,
        op: JournalOp::Commit { txn_id: 42 },
    };

    sink.journal_write(&entry).expect("journal write");

    let mut guard = sink.device.lock();
    let mut buf = vec![0u8; PAGE_SIZE];
    guard.read_block(1, &mut buf).expect("read journal slot");

    let mut seq_bytes = [0u8; 8];
    seq_bytes.copy_from_slice(&buf[..8]);
    assert_eq!(u64::from_le_bytes(seq_bytes), 3);

    let mut op_bytes = [0u8; 4];
    op_bytes.copy_from_slice(&buf[8..12]);
    assert_eq!(u32::from_le_bytes(op_bytes), 1);
}
