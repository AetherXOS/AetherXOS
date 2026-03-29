use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU32, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::modules::posix::PosixErrno;
use crate::modules::vfs::types::FileSystem;

#[derive(Debug, Clone, Copy)]
struct MmapState {
    prot: u32,
    flags: u32,
    len: usize,
}

lazy_static! {
    static ref MMAP_STATES: Mutex<BTreeMap<u32, MmapState>> = Mutex::new(BTreeMap::new());
    static ref LOCKED_MAPS: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
    static ref ANON_MAP_DATA: Mutex<BTreeMap<u32, Arc<Mutex<alloc::vec::Vec<u8>>>>> =
        Mutex::new(BTreeMap::new());
}

static NEXT_ANON_MAP_ID: AtomicU32 = AtomicU32::new(1_000_000);
static MLOCKALL_MODE: AtomicU32 = AtomicU32::new(0);

#[inline(always)]
fn valid_prot(prot: u32) -> bool {
    let allowed = crate::modules::posix_consts::mman::PROT_READ
        | crate::modules::posix_consts::mman::PROT_WRITE
        | crate::modules::posix_consts::mman::PROT_EXEC
        | crate::modules::posix_consts::mman::PROT_NONE;
    (prot & !allowed) == 0
}

#[inline(always)]
fn valid_flags(flags: u32) -> bool {
    let allowed = crate::modules::posix_consts::mman::MAP_SHARED
        | crate::modules::posix_consts::mman::MAP_PRIVATE
        | crate::modules::posix_consts::mman::MAP_ANONYMOUS;
    if (flags & !allowed) != 0 {
        return false;
    }
    let shared = (flags & crate::modules::posix_consts::mman::MAP_SHARED) != 0;
    let private = (flags & crate::modules::posix_consts::mman::MAP_PRIVATE) != 0;
    shared ^ private
}

struct BoxedVfsFile {
    inner: Box<dyn crate::modules::vfs::File>,
}

impl crate::modules::vfs::File for BoxedVfsFile {
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

pub fn mmap_file(
    fs_id: u32,
    path: &str,
    offset: usize,
    len: usize,
    prot: u32,
    flags: u32,
) -> Result<u32, PosixErrno> {
    if len == 0 || !valid_prot(prot) || !valid_flags(flags) {
        return Err(PosixErrno::Invalid);
    }

    let writable = (prot & crate::modules::posix_consts::mman::PROT_WRITE) != 0;
    let shared = (flags & crate::modules::posix_consts::mman::MAP_SHARED) != 0;
    let map_id = crate::modules::posix::fs::mmap(fs_id, path, offset, len, writable, shared)?;

    // True memory sharing is achieved via the SHARED_MAPPINGS registry in mmap_support.
    // When different processes call mmap on the same (fs_id, path, offset), they get the same Arc-wrapped buffer.

    MMAP_STATES
        .lock()
        .insert(map_id, MmapState { prot, flags, len });
    Ok(map_id)
}

pub fn shm_open(name: &str, oflag: i32, _mode: u32) -> Result<u32, PosixErrno> {
    if !name.starts_with('/') || name.len() < 2 || name[1..].contains('/') {
        return Err(PosixErrno::Invalid);
    }

    let shm_fs_id = *crate::modules::posix::fs::SHM_FS_ID;
    if shm_fs_id == 0 {
        return Err(PosixErrno::BadFileDescriptor);
    }

    let creat = (oflag & crate::modules::posix_consts::fs::O_CREAT) != 0;
    let excl = (oflag & crate::modules::posix_consts::fs::O_EXCL) != 0;
    let trunc = (oflag & crate::modules::posix_consts::fs::O_TRUNC) != 0;

    let internal_path = &name[1..];
    let tid = crate::interfaces::TaskId(crate::modules::posix::process::gettid());

    let (handle, path) = {
        let contexts = crate::modules::posix::fs::FS_CONTEXTS.lock();
        let fs = contexts
            .get(&shm_fs_id)
            .ok_or(PosixErrno::BadFileDescriptor)?;

        let exists = fs.stat(internal_path, tid).is_ok();

        if creat && excl && exists {
            return Err(PosixErrno::AlreadyExists);
        }
        if !creat && !exists {
            return Err(PosixErrno::NoEntry);
        }

        let mut handle = if creat && !exists {
            fs.create(internal_path, tid)
                .map_err(|_| PosixErrno::NoEntry)?
        } else {
            fs.open(internal_path, tid)
                .map_err(|_| PosixErrno::NoEntry)?
        };

        if trunc {
            let _ = handle.truncate(0);
        }

        (handle, String::from(internal_path))
    };

    let handle: Arc<Mutex<dyn crate::modules::vfs::File>> =
        Arc::new(Mutex::new(BoxedVfsFile { inner: handle }));
    let fd = crate::modules::posix::fs::register_handle(shm_fs_id, path, handle, true);

    Ok(fd)
}

pub fn shm_unlink(name: &str) -> Result<(), PosixErrno> {
    if !name.starts_with('/') || name.len() < 2 || name[1..].contains('/') {
        return Err(PosixErrno::Invalid);
    }

    let shm_fs_id = *crate::modules::posix::fs::SHM_FS_ID;
    let tid = crate::interfaces::TaskId(crate::modules::posix::process::gettid());

    let contexts = crate::modules::posix::fs::FS_CONTEXTS.lock();
    let fs = contexts
        .get(&shm_fs_id)
        .ok_or(PosixErrno::BadFileDescriptor)?;

    fs.remove(&name[1..], tid).map_err(|_| PosixErrno::NoEntry)
}

pub fn mmap_anonymous(len: usize, prot: u32, flags: u32) -> Result<u32, PosixErrno> {
    if len == 0 || !valid_prot(prot) {
        return Err(PosixErrno::Invalid);
    }
    let anon_flags = flags | crate::modules::posix_consts::mman::MAP_ANONYMOUS;
    if !valid_flags(anon_flags) {
        return Err(PosixErrno::Invalid);
    }

    let map_id = NEXT_ANON_MAP_ID.fetch_add(1, Ordering::Relaxed);
    MMAP_STATES.lock().insert(
        map_id,
        MmapState {
            prot,
            flags: anon_flags,
            len,
        },
    );
    ANON_MAP_DATA
        .lock()
        .insert(map_id, Arc::new(Mutex::new(alloc::vec![0u8; len])));
    Ok(map_id)
}

pub fn mremap(map_id: u32, new_len: usize) -> Result<(), PosixErrno> {
    if new_len == 0 {
        return Err(PosixErrno::Invalid);
    }
    let mut maps = MMAP_STATES.lock();
    let state = maps.get_mut(&map_id).ok_or(PosixErrno::BadFileDescriptor)?;
    state.len = new_len;
    Ok(())
}

pub fn mprotect(map_id: u32, prot: u32) -> Result<(), PosixErrno> {
    if !valid_prot(prot) {
        return Err(PosixErrno::Invalid);
    }
    let mut maps = MMAP_STATES.lock();
    let state = maps.get_mut(&map_id).ok_or(PosixErrno::BadFileDescriptor)?;
    state.prot = prot;
    Ok(())
}

pub fn madvise(map_id: u32, advice: i32) -> Result<(), PosixErrno> {
    let exists = MMAP_STATES.lock().contains_key(&map_id);
    if !exists {
        return Err(PosixErrno::BadFileDescriptor);
    }

    match advice {
        crate::modules::posix_consts::mman::MADV_NORMAL
        | crate::modules::posix_consts::mman::MADV_RANDOM
        | crate::modules::posix_consts::mman::MADV_SEQUENTIAL
        | crate::modules::posix_consts::mman::MADV_WILLNEED
        | crate::modules::posix_consts::mman::MADV_DONTNEED => Ok(()),
        _ => Err(PosixErrno::Invalid),
    }
}

pub fn mincore(map_id: u32) -> Result<bool, PosixErrno> {
    if MMAP_STATES.lock().contains_key(&map_id) {
        Ok(true)
    } else {
        Err(PosixErrno::BadFileDescriptor)
    }
}

pub fn mlock(map_id: u32) -> Result<(), PosixErrno> {
    if !MMAP_STATES.lock().contains_key(&map_id) {
        return Err(PosixErrno::BadFileDescriptor);
    }
    LOCKED_MAPS.lock().insert(map_id);
    Ok(())
}

pub fn munlock(map_id: u32) -> Result<(), PosixErrno> {
    if !MMAP_STATES.lock().contains_key(&map_id) {
        return Err(PosixErrno::BadFileDescriptor);
    }
    LOCKED_MAPS.lock().remove(&map_id);
    Ok(())
}

pub fn msync(map_id: u32) -> Result<(), PosixErrno> {
    if !MMAP_STATES.lock().contains_key(&map_id) {
        return Err(PosixErrno::BadFileDescriptor);
    }
    if ANON_MAP_DATA.lock().contains_key(&map_id) {
        return Ok(());
    }
    crate::modules::posix::fs::msync(map_id)
}

pub fn msync_flags(map_id: u32, flags: u32) -> Result<(), PosixErrno> {
    let allowed = crate::modules::posix_consts::mman::MS_ASYNC
        | crate::modules::posix_consts::mman::MS_INVALIDATE
        | crate::modules::posix_consts::mman::MS_SYNC;
    if (flags & !allowed) != 0 {
        return Err(PosixErrno::Invalid);
    }

    let sync = (flags & crate::modules::posix_consts::mman::MS_SYNC) != 0;
    let async_mode = (flags & crate::modules::posix_consts::mman::MS_ASYNC) != 0;
    if sync && async_mode {
        return Err(PosixErrno::Invalid);
    }

    msync(map_id)
}

pub fn msync_range(map_id: u32, offset: usize, len: usize) -> Result<(), PosixErrno> {
    let state = *MMAP_STATES
        .lock()
        .get(&map_id)
        .ok_or(PosixErrno::BadFileDescriptor)?;
    if len == 0 {
        return Ok(());
    }
    let end = offset.checked_add(len).ok_or(PosixErrno::Invalid)?;
    if end > state.len {
        return Err(PosixErrno::Invalid);
    }
    crate::modules::posix::fs::msync(map_id)
}

pub fn mmap_read(map_id: u32, dst: &mut [u8], map_offset: usize) -> Result<usize, PosixErrno> {
    if !can_read(map_id)? {
        return Err(PosixErrno::PermissionDenied);
    }

    let map_len = mapped_len(map_id)?;
    if map_offset >= map_len {
        return Ok(0);
    }
    if let Some(arc) = ANON_MAP_DATA.lock().get(&map_id).cloned() {
        let data = arc.lock();
        let end = core::cmp::min(map_offset.saturating_add(dst.len()), data.len());
        let read_len = end.saturating_sub(map_offset);
        if read_len == 0 {
            return Ok(0);
        }
        dst[..read_len].copy_from_slice(&data[map_offset..end]);
        return Ok(read_len);
    }

    crate::modules::posix::fs::mmap_read(map_id, dst, map_offset)
}

pub fn mmap_write(map_id: u32, src: &[u8], map_offset: usize) -> Result<usize, PosixErrno> {
    if !can_write(map_id)? {
        return Err(PosixErrno::PermissionDenied);
    }

    let map_len = mapped_len(map_id)?;
    if map_offset >= map_len {
        return Ok(0);
    }
    {
        let anon = ANON_MAP_DATA.lock();
        if let Some(arc) = anon.get(&map_id) {
            let mut data = arc.lock();
            let end = core::cmp::min(map_offset.saturating_add(src.len()), data.len());
            let write_len = end.saturating_sub(map_offset);
            if write_len == 0 {
                return Ok(0);
            }
            data[map_offset..end].copy_from_slice(&src[..write_len]);
            return Ok(write_len);
        }
    }

    crate::modules::posix::fs::mmap_write(map_id, src, map_offset)
}

pub fn munmap(map_id: u32) -> Result<(), PosixErrno> {
    let is_anon = ANON_MAP_DATA.lock().contains_key(&map_id);
    MMAP_STATES
        .lock()
        .remove(&map_id)
        .ok_or(PosixErrno::BadFileDescriptor)?;
    LOCKED_MAPS.lock().remove(&map_id);
    ANON_MAP_DATA.lock().remove(&map_id);
    if is_anon {
        return Ok(());
    }
    crate::modules::posix::fs::munmap(map_id)
}

pub fn get_prot(map_id: u32) -> Result<u32, PosixErrno> {
    MMAP_STATES
        .lock()
        .get(&map_id)
        .map(|m| m.prot)
        .ok_or(PosixErrno::BadFileDescriptor)
}

pub fn get_flags(map_id: u32) -> Result<u32, PosixErrno> {
    MMAP_STATES
        .lock()
        .get(&map_id)
        .map(|m| m.flags)
        .ok_or(PosixErrno::BadFileDescriptor)
}

pub fn is_locked(map_id: u32) -> Result<bool, PosixErrno> {
    if !MMAP_STATES.lock().contains_key(&map_id) {
        return Err(PosixErrno::BadFileDescriptor);
    }
    Ok(LOCKED_MAPS.lock().contains(&map_id))
}

pub fn mapped_len(map_id: u32) -> Result<usize, PosixErrno> {
    MMAP_STATES
        .lock()
        .get(&map_id)
        .map(|m| m.len)
        .ok_or(PosixErrno::BadFileDescriptor)
}

pub fn can_read(map_id: u32) -> Result<bool, PosixErrno> {
    let prot = get_prot(map_id)?;
    Ok((prot & crate::modules::posix_consts::mman::PROT_READ) != 0)
}

pub fn can_write(map_id: u32) -> Result<bool, PosixErrno> {
    let prot = get_prot(map_id)?;
    Ok((prot & crate::modules::posix_consts::mman::PROT_WRITE) != 0)
}

pub fn can_exec(map_id: u32) -> Result<bool, PosixErrno> {
    let prot = get_prot(map_id)?;
    Ok((prot & crate::modules::posix_consts::mman::PROT_EXEC) != 0)
}

pub fn mlockall(flags: u32) -> Result<(), PosixErrno> {
    let allowed = crate::modules::posix_consts::mman::MCL_CURRENT
        | crate::modules::posix_consts::mman::MCL_FUTURE;
    if flags == 0 || (flags & !allowed) != 0 {
        return Err(PosixErrno::Invalid);
    }

    MLOCKALL_MODE.store(flags, Ordering::Relaxed);
    if (flags & crate::modules::posix_consts::mman::MCL_CURRENT) != 0 {
        let ids: alloc::vec::Vec<u32> = MMAP_STATES.lock().keys().copied().collect();
        let mut locked = LOCKED_MAPS.lock();
        for id in ids {
            locked.insert(id);
        }
    }
    Ok(())
}

pub fn munlockall() {
    MLOCKALL_MODE.store(0, Ordering::Relaxed);
    LOCKED_MAPS.lock().clear();
}

#[inline(always)]
pub fn mlockall_mode() -> u32 {
    MLOCKALL_MODE.load(Ordering::Relaxed)
}
