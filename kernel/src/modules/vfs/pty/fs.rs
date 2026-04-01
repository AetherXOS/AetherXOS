use super::*;
use crate::interfaces::TaskId;
use crate::modules::vfs::FileSystem;

pub struct PtsFs;

impl PtsFs {
    pub fn new() -> Self {
        Self
    }
}

impl FileSystem for PtsFs {
    fn open(&self, path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        let clean = path.trim_start_matches('/');

        if clean == "ptmx" {
            return open_ptmx();
        }

        let idx: u32 = clean.parse().map_err(|_| "ENOENT")?;
        open_pts(idx)
    }

    fn create(&self, _path: &str, _tid: TaskId) -> Result<Box<dyn File>, &'static str> {
        Err("EROFS")
    }

    fn remove(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("EROFS")
    }

    fn mkdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("EROFS")
    }

    fn rmdir(&self, _path: &str, _tid: TaskId) -> Result<(), &'static str> {
        Err("EROFS")
    }

    fn readdir(&self, _path: &str, _tid: TaskId) -> Result<Vec<DirEntry>, &'static str> {
        let indices = super::registry::list_ptys();
        let mut entries = vec![DirEntry {
            name: "ptmx".to_string(),
            ino: 1,
            kind: 2,
        }];

        for idx in indices {
            entries.push(DirEntry {
                name: format!("{}", idx),
                ino: (10 + idx) as u64,
                kind: 2,
            });
        }

        Ok(entries)
    }

    fn stat(&self, path: &str, _tid: TaskId) -> Result<FileStats, &'static str> {
        let clean = path.trim_start_matches('/');
        if clean.is_empty() {
            return Ok(FileStats {
                size: 0,
                mode: 0o040755,
                uid: 0,
                gid: 5,
                atime: 0,
                mtime: 0,
                ctime: 0,
                blksize: 4096,
                blocks: 0,
            });
        }

        Ok(FileStats {
            size: 0,
            mode: 0o020620,
            uid: 0,
            gid: 5,
            atime: 0,
            mtime: 0,
            ctime: 0,
            blksize: 4096,
            blocks: 0,
        })
    }
}
