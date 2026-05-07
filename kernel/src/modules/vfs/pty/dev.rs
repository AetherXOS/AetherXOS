use super::*;
use core::any::Any;

/// `/dev/ptmx` — opening this allocates a new PTY pair.
pub struct DevPtmx;

impl File for DevPtmx {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("EIO")
    }

    fn write(&mut self, _buf: &[u8]) -> Result<usize, &'static str> {
        Err("EIO")
    }

    fn stat(&self) -> Result<FileStats, &'static str> {
        Ok(FileStats {
            size: 0,
            mode: 0o020666,
            uid: 0,
            gid: 5,
            atime: Default::default(),
            mtime: Default::default(),
            ctime: Default::default(),
            blocks: 0,
            ..Default::default()
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Open /dev/ptmx: allocates a PTY pair and returns the master file.
pub fn open_ptmx() -> Result<Box<dyn File>, &'static str> {
    let (index, pair) = super::registry::allocate_pty().ok_or("ENOMEM")?;
    Ok(Box::new(PtyMaster::new(index, pair)))
}

/// Open /dev/pts/N: returns the slave side of PTY pair N.
pub fn open_pts(index: u32) -> Result<Box<dyn File>, &'static str> {
    let pair = super::registry::get_pty_pair(index).ok_or("ENOENT")?;
    if pair.is_locked() {
        return Err("EIO");
    }
    pair.set_slave_opened();
    Ok(Box::new(PtySlave::new(index, pair)))
}
