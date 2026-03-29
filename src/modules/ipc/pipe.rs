/// Unix pipes — unidirectional byte-stream communication between processes.
///
/// Supports both anonymous pipes (pipe() syscall) and named pipes (FIFOs).
/// Uses a fixed-size ring buffer internally.
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Default pipe buffer size (64 KiB, same as Linux).
const PIPE_BUF_SIZE: usize = 65536;

/// POSIX PIPE_BUF — writes of this size or less are guaranteed atomic.
const _PIPE_BUF_ATOMIC: usize = 4096;

/// Pipe identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PipeId(pub u64);

static NEXT_PIPE_ID: AtomicU64 = AtomicU64::new(1);

fn alloc_pipe_id() -> PipeId {
    PipeId(NEXT_PIPE_ID.fetch_add(1, Ordering::Relaxed))
}

/// Internal ring buffer for pipe data.
struct PipeBuffer {
    buf: Vec<u8>,
    head: usize, // read position
    tail: usize, // write position
    count: usize,
}

impl PipeBuffer {
    fn new(capacity: usize) -> Self {
        Self {
            buf: {
                let mut v = Vec::with_capacity(capacity);
                v.resize(capacity, 0);
                v
            },
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn _capacity(&self) -> usize {
        self.buf.len()
    }

    fn available_read(&self) -> usize {
        self.count
    }

    fn available_write(&self) -> usize {
        self.buf.len() - self.count
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn is_full(&self) -> bool {
        self.count == self.buf.len()
    }

    /// Write data into the pipe. Returns number of bytes written.
    fn write(&mut self, data: &[u8]) -> usize {
        let to_write = data.len().min(self.available_write());
        for i in 0..to_write {
            self.buf[self.tail] = data[i];
            self.tail = (self.tail + 1) % self.buf.len();
        }
        self.count += to_write;
        to_write
    }

    /// Read data from the pipe. Returns number of bytes read.
    fn read(&mut self, out: &mut [u8]) -> usize {
        let to_read = out.len().min(self.available_read());
        for i in 0..to_read {
            out[i] = self.buf[self.head];
            self.head = (self.head + 1) % self.buf.len();
        }
        self.count -= to_read;
        to_read
    }
}

/// Pipe state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeEnd {
    Read,
    Write,
}

/// A pipe instance.
struct PipeInner {
    _id: PipeId,
    buffer: PipeBuffer,
    /// Number of open read-end descriptors.
    readers: u32,
    /// Number of open write-end descriptors.
    writers: u32,
    /// Whether this is a named pipe (FIFO).
    is_named: bool,
    /// Path for named pipes.
    path: Option<String>,
}

impl PipeInner {
    fn new(id: PipeId, capacity: usize) -> Self {
        Self {
            _id: id,
            buffer: PipeBuffer::new(capacity),
            readers: 1,
            writers: 1,
            is_named: false,
            path: None,
        }
    }

    fn new_named(id: PipeId, path: String, capacity: usize) -> Self {
        Self {
            _id: id,
            buffer: PipeBuffer::new(capacity),
            readers: 0, // opened lazily
            writers: 0,
            is_named: true,
            path: Some(path),
        }
    }
}

/// Global pipe registry.
pub struct PipeRegistry {
    pipes: BTreeMap<PipeId, PipeInner>,
    /// Named pipe path → PipeId.
    named_pipes: BTreeMap<String, PipeId>,
    /// Stats.
    total_created: u64,
    total_bytes_written: u64,
    total_bytes_read: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct PipeStats {
    pub total_created: u64,
    pub total_bytes_written: u64,
    pub total_bytes_read: u64,
    pub active_pipes: usize,
    pub named_pipes: usize,
}

impl PipeRegistry {
    pub fn new() -> Self {
        Self {
            pipes: BTreeMap::new(),
            named_pipes: BTreeMap::new(),
            total_created: 0,
            total_bytes_written: 0,
            total_bytes_read: 0,
        }
    }

    /// Create an anonymous pipe. Returns (pipe_id).
    pub fn pipe(&mut self) -> PipeId {
        let id = alloc_pipe_id();
        self.pipes.insert(id, PipeInner::new(id, PIPE_BUF_SIZE));
        self.total_created += 1;
        id
    }

    /// Create a named pipe (FIFO) at the given path.
    pub fn mkfifo(&mut self, path: &str) -> Result<PipeId, &'static str> {
        if self.named_pipes.contains_key(path) {
            return Err("FIFO already exists");
        }
        let id = alloc_pipe_id();
        self.pipes.insert(
            id,
            PipeInner::new_named(id, String::from(path), PIPE_BUF_SIZE),
        );
        self.named_pipes.insert(String::from(path), id);
        self.total_created += 1;
        Ok(id)
    }

    /// Open a named pipe for reading or writing.
    pub fn open_fifo(&mut self, path: &str, end: PipeEnd) -> Result<PipeId, &'static str> {
        let id = self
            .named_pipes
            .get(path)
            .copied()
            .ok_or("FIFO not found")?;
        let pipe = self.pipes.get_mut(&id).ok_or("pipe not found")?;
        match end {
            PipeEnd::Read => pipe.readers += 1,
            PipeEnd::Write => pipe.writers += 1,
        }
        Ok(id)
    }

    /// Write to a pipe. Returns number of bytes written or error.
    pub fn write(&mut self, id: PipeId, data: &[u8]) -> Result<usize, &'static str> {
        let pipe = self.pipes.get_mut(&id).ok_or("pipe not found")?;
        if pipe.readers == 0 {
            return Err("EPIPE: no readers");
        }
        if pipe.buffer.is_full() {
            return Err("EAGAIN: pipe full");
        }
        let written = pipe.buffer.write(data);
        self.total_bytes_written += written as u64;
        Ok(written)
    }

    /// Read from a pipe. Returns number of bytes read.
    pub fn read(&mut self, id: PipeId, buf: &mut [u8]) -> Result<usize, &'static str> {
        let pipe = self.pipes.get_mut(&id).ok_or("pipe not found")?;
        if pipe.buffer.is_empty() {
            if pipe.writers == 0 {
                return Ok(0); // EOF
            }
            return Err("EAGAIN: pipe empty");
        }
        let read = pipe.buffer.read(buf);
        self.total_bytes_read += read as u64;
        Ok(read)
    }

    /// Close one end of a pipe.
    pub fn close(&mut self, id: PipeId, end: PipeEnd) {
        if let Some(pipe) = self.pipes.get_mut(&id) {
            match end {
                PipeEnd::Read => pipe.readers = pipe.readers.saturating_sub(1),
                PipeEnd::Write => pipe.writers = pipe.writers.saturating_sub(1),
            }
            // Garbage collect if both ends closed and not named
            if pipe.readers == 0 && pipe.writers == 0 && !pipe.is_named {
                self.pipes.remove(&id);
            }
        }
    }

    /// Remove a named pipe (unlink).
    pub fn unlink_fifo(&mut self, path: &str) -> Result<(), &'static str> {
        let id = self.named_pipes.remove(path).ok_or("FIFO not found")?;
        // Mark it for destruction when all fds close
        if let Some(pipe) = self.pipes.get_mut(&id) {
            pipe.is_named = false;
            pipe.path = None;
            if pipe.readers == 0 && pipe.writers == 0 {
                self.pipes.remove(&id);
            }
        }
        Ok(())
    }

    /// Query how many bytes are available to read.
    pub fn available(&self, id: PipeId) -> Result<usize, &'static str> {
        let pipe = self.pipes.get(&id).ok_or("pipe not found")?;
        Ok(pipe.buffer.available_read())
    }

    pub fn stats(&self) -> PipeStats {
        PipeStats {
            total_created: self.total_created,
            total_bytes_written: self.total_bytes_written,
            total_bytes_read: self.total_bytes_read,
            active_pipes: self.pipes.len(),
            named_pipes: self.named_pipes.len(),
        }
    }
}

/// Global pipe registry.
static GLOBAL_PIPES: Mutex<Option<PipeRegistry>> = Mutex::new(None);

/// Initialize the global pipe registry.
pub fn init_pipes() {
    let mut guard = GLOBAL_PIPES.lock();
    if guard.is_none() {
        *guard = Some(PipeRegistry::new());
    }
}

/// Create an anonymous pipe.
pub fn pipe() -> Result<PipeId, &'static str> {
    let mut guard = GLOBAL_PIPES.lock();
    Ok(guard.as_mut().ok_or("pipes not initialized")?.pipe())
}

/// Create a named pipe (FIFO).
pub fn mkfifo(path: &str) -> Result<PipeId, &'static str> {
    let mut guard = GLOBAL_PIPES.lock();
    guard.as_mut().ok_or("pipes not initialized")?.mkfifo(path)
}

/// Write to a pipe.
pub fn pipe_write(id: PipeId, data: &[u8]) -> Result<usize, &'static str> {
    let mut guard = GLOBAL_PIPES.lock();
    guard
        .as_mut()
        .ok_or("pipes not initialized")?
        .write(id, data)
}

/// Read from a pipe.
pub fn pipe_read(id: PipeId, buf: &mut [u8]) -> Result<usize, &'static str> {
    let mut guard = GLOBAL_PIPES.lock();
    guard.as_mut().ok_or("pipes not initialized")?.read(id, buf)
}

/// Close one end of a pipe.
pub fn pipe_close(id: PipeId, end: PipeEnd) {
    let mut guard = GLOBAL_PIPES.lock();
    if let Some(ref mut reg) = *guard {
        reg.close(id, end);
    }
}
