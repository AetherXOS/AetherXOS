use super::{
    access, map_fs_error, normalize_path, PosixErrno, PosixMapDesc, FS_CONTEXTS, MMAP_TABLE,
    NEXT_MAP_ID,
};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    /// Tracks SHARED mappings by (fs_id, path, offset) to allow true memory sharing.
    static ref SHARED_MAPPINGS: Mutex<BTreeMap<(u32, String, usize), Arc<Mutex<Vec<u8>>>>> = Mutex::new(BTreeMap::new());
}

pub fn mmap(
    fs_id: u32,
    path: &str,
    offset: usize,
    len: usize,
    writable: bool,
    shared: bool,
) -> Result<u32, PosixErrno> {
    let path_n = normalize_path(path)?;
    if !access(fs_id, &path_n)? {
        return Err(PosixErrno::NoEntry);
    }

    let buffer = {
        let fd = super::open(fs_id, path, false)?;
        let shared = super::get_file_description(fd)?;
        let _ = super::close(fd);
        let mapped = shared
            .handle
            .lock()
            .mmap(offset as u64, len)
            .map_err(map_fs_error)?;
        mapped
    };

    let map_id = NEXT_MAP_ID.fetch_add(1, Ordering::Relaxed);
    MMAP_TABLE.lock().insert(
        map_id,
        PosixMapDesc {
            fs_id,
            path: path_n,
            offset,
            len,
            writable,
            dirty: false,
            data: buffer,
            shared,
        },
    );
    Ok(map_id)
}

pub fn mmap_read(map_id: u32, dst: &mut [u8], map_offset: usize) -> Result<usize, PosixErrno> {
    let table = MMAP_TABLE.lock();
    let map = table.get(&map_id).ok_or(PosixErrno::BadFileDescriptor)?;
    if map_offset >= map.len {
        return Ok(0);
    }
    let data = map.data.lock();
    let end = core::cmp::min(map_offset.saturating_add(dst.len()), map.len);
    let read_len = end - map_offset;
    dst[..read_len].copy_from_slice(&data[map_offset..end]);
    Ok(read_len)
}

pub fn mmap_write(map_id: u32, src: &[u8], map_offset: usize) -> Result<usize, PosixErrno> {
    let mut table = MMAP_TABLE.lock();
    let map = table
        .get_mut(&map_id)
        .ok_or(PosixErrno::BadFileDescriptor)?;
    if !map.writable {
        return Err(PosixErrno::PermissionDenied);
    }
    if map_offset >= map.len {
        return Ok(0);
    }
    let mut data = map.data.lock();
    let end = core::cmp::min(map_offset.saturating_add(src.len()), map.len);
    let write_len = end - map_offset;
    data[map_offset..end].copy_from_slice(&src[..write_len]);
    map.dirty = true;
    Ok(write_len)
}

fn flush_map(map_id: u32, remove_after: bool) -> Result<(), PosixErrno> {
    let snapshot = {
        let table = MMAP_TABLE.lock();
        table
            .get(&map_id)
            .cloned()
            .ok_or(PosixErrno::BadFileDescriptor)?
    };

    if snapshot.dirty {
        let contexts = FS_CONTEXTS.lock();
        let fs = contexts
            .get(&snapshot.fs_id)
            .ok_or(PosixErrno::BadFileDescriptor)?;
        let mut file_data = fs.read_all(&snapshot.path).unwrap_or_default();
        let required_len = snapshot.offset.saturating_add(snapshot.len);
        if file_data.len() < required_len {
            file_data.resize(required_len, 0);
        }
        let end = snapshot.offset + snapshot.len;
        let data = snapshot.data.lock();
        file_data[snapshot.offset..end].copy_from_slice(&data);
        let _ = fs
            .write_all(&snapshot.path, &file_data)
            .map_err(map_fs_error)?;
    }

    if remove_after {
        MMAP_TABLE.lock().remove(&map_id);
    } else if snapshot.dirty {
        let mut table = MMAP_TABLE.lock();
        if let Some(map) = table.get_mut(&map_id) {
            map.dirty = false;
        }
    }
    Ok(())
}

pub fn msync(map_id: u32) -> Result<(), PosixErrno> {
    flush_map(map_id, false)
}

pub fn munmap(map_id: u32) -> Result<(), PosixErrno> {
    flush_map(map_id, true)
}
