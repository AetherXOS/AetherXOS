use super::*;

#[allow(dead_code)]
pub(super) struct StatelessDevice {
    pub(super) fill: u8,
    pub(super) is_null: bool,
}

impl crate::modules::vfs::File for StatelessDevice {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        if self.is_null {
            Ok(0)
        } else {
            buf.fill(self.fill);
            Ok(buf.len())
        }
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        Ok(buf.len())
    }
    fn seek(&mut self, _pos: crate::modules::vfs::SeekFrom) -> Result<u64, &'static str> {
        Ok(0)
    }
    fn flush(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}

pub(super) struct BoxedFile {
    pub(super) inner: Box<dyn crate::modules::vfs::File>,
}

impl crate::modules::vfs::File for BoxedFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, &'static str> {
        self.inner.read(buf)
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, &'static str> {
        self.inner.write(buf)
    }
    fn seek(&mut self, pos: crate::modules::vfs::SeekFrom) -> Result<u64, &'static str> {
        self.inner.seek(pos)
    }
    fn flush(&mut self) -> Result<(), &'static str> {
        self.inner.flush()
    }
    fn truncate(&mut self, size: u64) -> Result<(), &'static str> {
        self.inner.truncate(size)
    }
    fn stat(&self) -> Result<crate::modules::vfs::types::FileStats, &'static str> {
        self.inner.stat()
    }
    fn poll_events(&self) -> crate::modules::vfs::types::PollEvents {
        self.inner.poll_events()
    }
    fn ioctl(&mut self, cmd: u32, arg: u64) -> Result<isize, &'static str> {
        self.inner.ioctl(cmd, arg)
    }
    fn mmap(
        &self,
        offset: u64,
        len: usize,
    ) -> Result<Arc<Mutex<alloc::vec::Vec<u8>>>, &'static str> {
        self.inner.mmap(offset, len)
    }
    fn as_any(&self) -> &dyn core::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }
}