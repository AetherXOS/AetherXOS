use crate::modules::vfs::types::FileStats;

/// VfsHelper: Shared logic for filesystem implementers.
/// Reduces code duplication across ramfs, devfs, procfs, etc.
pub trait VfsHelper {
    /// Generic bounds check for read/write operations.
    fn check_bounds(offset: u64, size: u64, count: usize) -> Result<usize, &'static str> {
        if offset >= size {
            return Ok(0);
        }
        let available = (size - offset) as usize;
        Ok(available.min(count))
    }

    /// Generic non-blocking logic wrapper.
    fn handle_nonblock<F, T>(nonblock: bool, mut f: F) -> Result<T, &'static str>
    where
        F: FnMut() -> Option<T>,
    {
        if let Some(res) = f() {
            Ok(res)
        } else if nonblock {
            Err("would block")
        } else {
            // Real blocking would happen here
            Err("resource unavailable")
        }
    }
}

pub struct GenericFile {
    pub stats: FileStats,
}

impl VfsHelper for GenericFile {}
