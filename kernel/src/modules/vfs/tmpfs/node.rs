//! Directory entry types for tmpfs.

use alloc::string::String;
use alloc::sync::Arc;
use spin::Mutex;
use super::data::TmpFileData;

#[derive(Clone)]
pub enum TmpNode {
    File(Arc<Mutex<TmpFileData>>),
    Dir { mode: u32, uid: u32, gid: u32 },
    Symlink(String),
}
