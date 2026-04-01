use super::*;

/// Simple in-memory sink (useful for testing the writeback pipeline).
pub struct RamWritebackSink {
    pages: Mutex<BTreeMap<(u64, u64), Vec<u8>>>,
}

impl RamWritebackSink {
    pub fn new() -> Self {
        Self {
            pages: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn read_back(&self, ino: u64, offset: u64) -> Option<Vec<u8>> {
        let key = (ino, offset / PAGE_SIZE as u64);
        self.pages.lock().get(&key).cloned()
    }
}

impl WritebackSink for RamWritebackSink {
    fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str> {
        let key = (ino, offset / PAGE_SIZE as u64);
        self.pages.lock().insert(key, data.to_vec());
        Ok(())
    }

    fn flush(&self) -> Result<(), &'static str> {
        Ok(())
    }
}
