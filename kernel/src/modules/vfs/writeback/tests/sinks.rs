use core::sync::atomic::{AtomicU64, Ordering};
use super::super::{WritebackSink, JournalEntry};
use spin::Mutex as SpinMutex;

pub struct RecordingSink {
    pub writes: SpinMutex<Vec<(u64, u64, usize)>>,
    pub flushes: AtomicU64,
}

impl RecordingSink {
    pub fn new() -> Self {
        Self {
            writes: SpinMutex::new(Vec::new()),
            flushes: AtomicU64::new(0),
        }
    }
}

impl WritebackSink for RecordingSink {
    fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str> {
        self.writes.lock().push((ino, offset, data.len()));
        Ok(())
    }

    fn flush(&self) -> Result<(), &'static str> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

pub struct FailingSink;

impl WritebackSink for FailingSink {
    fn write_page(&self, _ino: u64, _offset: u64, _data: &[u8]) -> Result<(), &'static str> {
        Err("injected write failure")
    }

    fn flush(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

pub struct TransactionalRecordingSink {
    pub writes: SpinMutex<Vec<(u64, u64, usize)>>,
    pub journal_entries: SpinMutex<Vec<u64>>,
    pub journal_commits: AtomicU64,
    pub flushes: AtomicU64,
}

impl TransactionalRecordingSink {
    pub fn new() -> Self {
        Self {
            writes: SpinMutex::new(Vec::new()),
            journal_entries: SpinMutex::new(Vec::new()),
            journal_commits: AtomicU64::new(0),
            flushes: AtomicU64::new(0),
        }
    }
}

impl WritebackSink for TransactionalRecordingSink {
    fn write_page(&self, ino: u64, offset: u64, data: &[u8]) -> Result<(), &'static str> {
        self.writes.lock().push((ino, offset, data.len()));
        Ok(())
    }

    fn flush(&self) -> Result<(), &'static str> {
        self.flushes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn journal_write(&self, entry: &JournalEntry) -> Result<(), &'static str> {
        self.journal_entries.lock().push(entry.seq);
        Ok(())
    }

    fn journal_commit(&self) -> Result<(), &'static str> {
        self.journal_commits.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}
