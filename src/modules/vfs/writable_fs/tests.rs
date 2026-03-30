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

#[test_case]
fn overlay_copy_up_persists_into_tmpfs_upper() {
    use crate::interfaces::TaskId;
    use crate::modules::vfs::{WritableOverlayFs, RamWritebackSink, overlay_registry, tmpfs};

    // Prepare a base filesystem and populate a file
    let mut base = tmpfs::TmpFs::new();
    {
        let mut f = base.create("/greeting", TaskId(0)).expect("create base file");
        f.write(b"hello").expect("write base");
    }

    // Create overlay instance with the base moved in and a RAM sink.
    let mount_id = 42usize;
    let sink = alloc::sync::Arc::new(RamWritebackSink::new());
    let overlay = WritableOverlayFs::new(base, mount_id, sink.clone());

    // Register overlay with a tmpfs upper so copy-up will write into it.
    let upper = tmpfs::TmpFs::new();
    overlay_registry::register_overlay_with_upper(mount_id, Box::new(overlay), Some(Box::new(upper)));

    // Open the file through the overlay (triggers copy-up)
    overlay_registry::with_overlay(mount_id, |fs| {
        let mut of = fs.open("/greeting", TaskId(0)).expect("open overlay");
        let mut buf = [0u8; 16];
        let n = of.read(&mut buf).expect("read overlay");
        assert_eq!(&buf[..n], b"hello");
        Ok(())
    })
    .expect("overlay open op");

    // Verify the upper tmpfs contains the copied data
    overlay_registry::with_upper(mount_id, |upper_opt| {
        let upper = upper_opt.ok_or("no upper registered")?;
        let mut uf = upper.open("/greeting", TaskId(0)).expect("open upper");
        let mut ubuf = [0u8; 16];
        let m = uf.read(&mut ubuf).expect("read upper");
        assert_eq!(&ubuf[..m], b"hello");
        Ok(())
    })
    .expect("upper check");
}
