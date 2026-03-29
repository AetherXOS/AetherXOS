use super::*;

pub fn devfs_event_snapshot(fs_id: u32) -> Result<DevFsEventSnapshot, PosixErrno> {
    let Some(devfs) = devfs_context(fs_id) else {
        return Err(PosixErrno::BadFileDescriptor);
    };
    sync_devfs_runtime_nodes(&devfs);
    Ok(devfs.events_snapshot())
}

pub fn devfs_events_since(
    fs_id: u32,
    after_seq: u64,
    max_items: usize,
) -> Result<alloc::vec::Vec<DevFsEvent>, PosixErrno> {
    let Some(devfs) = devfs_context(fs_id) else {
        return Err(PosixErrno::BadFileDescriptor);
    };
    sync_devfs_runtime_nodes(&devfs);
    Ok(devfs.events_since(after_seq, max_items.max(1)))
}