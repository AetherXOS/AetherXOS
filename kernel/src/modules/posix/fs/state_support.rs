use super::*;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref FS_CONTEXTS: Mutex<BTreeMap<u32, crate::modules::vfs::disk_fs::DiskFsLibrary>> =
        Mutex::new(BTreeMap::new());
    pub static ref FILE_TABLE: Mutex<BTreeMap<u32, PosixFileDesc>> = Mutex::new(BTreeMap::new());
    pub static ref DIR_TABLE: Mutex<BTreeMap<u32, VecDeque<String>>> = Mutex::new(BTreeMap::new());
    pub static ref CWD_INDEX: Mutex<BTreeMap<u32, String>> = Mutex::new(BTreeMap::new());
    pub static ref FILE_INDEX: Mutex<BTreeMap<u32, BTreeSet<String>>> = Mutex::new(BTreeMap::new());
    pub static ref MMAP_TABLE: Mutex<BTreeMap<u32, PosixMapDesc>> = Mutex::new(BTreeMap::new());
    pub static ref DEVFS_CONTEXTS: Mutex<BTreeMap<u32, Arc<DevFs>>> = Mutex::new(BTreeMap::new());
    pub static ref SHM_FS_ID: u32 = mount_ramfs("/dev/shm").unwrap_or(0);
}

pub static NEXT_FS_ID: AtomicU32 = AtomicU32::new(1);
pub static NEXT_FD: AtomicU32 = AtomicU32::new(1000);
pub static NEXT_DIRFD: AtomicU32 = AtomicU32::new(20000);
pub static NEXT_MAP_ID: AtomicU32 = AtomicU32::new(40000);
pub static UMASK_BITS: AtomicU32 = AtomicU32::new(0o022);
pub const POSIX_SUPPORTED_STATUS_FLAGS: u32 = (crate::modules::posix_consts::fs::O_APPEND as u32)
    | (crate::modules::posix_consts::net::O_NONBLOCK as u32);
pub const POSIX_DESCRIPTOR_CLOEXEC: u32 = 0x1;