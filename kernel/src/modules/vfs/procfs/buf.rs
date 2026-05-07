use super::*;

/// A simple read-only in-memory file backed by a dynamically generated buffer.
pub struct ReadOnlyBuf {
    pub data: Vec<u8>,
    pub pos: usize,
}

impl ReadOnlyBuf {
    pub fn from_string(s: String) -> Self {
        Self {
            data: s.into_bytes(),
            pos: 0,
        }
    }
}

impl File for ReadOnlyBuf {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        let remaining = &self.data[self.pos..];
        let n = buf.len().min(remaining.len());
        buf[..n].copy_from_slice(&remaining[..n]);
        self.pos += n;
        Ok(n)
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("EROFS")
    }

    fn seek(&mut self, pos: crate::modules::vfs::types::SeekFrom) -> Result<u64, &'static str> {
        match pos {
            crate::modules::vfs::types::SeekFrom::Start(n) => {
                self.pos = n as usize;
            }
            crate::modules::vfs::types::SeekFrom::Current(n) => {
                self.pos = (self.pos as i64 + n) as usize;
            }
            crate::modules::vfs::types::SeekFrom::End(n) => {
                self.pos = (self.data.len() as i64 + n) as usize;
            }
        }
        Ok(self.pos as u64)
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: self.data.len() as u64,
            mode: 0o100444, // regular file, r--r--r--
            uid: 0,
            gid: 0,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            blksize: 4096,
            blocks: 0,`n        })
    }

    fn poll_events(&self) -> PollEvents {
        PollEvents::IN
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
