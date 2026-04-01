extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use spin::Mutex;

use crate::modules::vfs::FileSystem;

/// Simple in-memory registry for writable overlay filesystem instances.
///
/// Stores boxed `FileSystem` instances keyed by `mount_id`. Each entry may
/// optionally include an `upper` filesystem (for tmpfs upper backing used
/// by copy-up). When unregistered we also attempt to unregister the
/// writeback sink.
static OVERLAY_INSTANCES: Mutex<Vec<(usize, Box<dyn FileSystem>, Option<Box<dyn FileSystem>>)>> =
    Mutex::new(Vec::new());

pub fn register_overlay(mount_id: usize, fs: Box<dyn FileSystem>) {
    register_overlay_with_upper(mount_id, fs, None);
}

pub fn register_overlay_with_upper(
    mount_id: usize,
    fs: Box<dyn FileSystem>,
    upper: Option<Box<dyn FileSystem>>,
) {
    let mut guard = OVERLAY_INSTANCES.lock();
    guard.push((mount_id, fs, upper));
}

pub fn unregister_overlay(mount_id: usize) -> Result<(), &'static str> {
    let mut guard = OVERLAY_INSTANCES.lock();
    if let Some(pos) = guard.iter().position(|(id, _, _)| *id == mount_id) {
        guard.remove(pos);
    }

    // Best-effort: unregister writeback sink for this mount.
    let _ = crate::modules::vfs::writeback::unregister_writable_mount(mount_id);
    Ok(())
}

/// Invoke a closure with the overlay filesystem instance for `mount_id`.
/// Returns `Err("mount not found")` if no overlay instance is registered.
pub fn with_overlay<T>(mount_id: usize, op: impl FnOnce(&dyn FileSystem) -> Result<T, &'static str>) -> Result<T, &'static str> {
    let guard = OVERLAY_INSTANCES.lock();
    if let Some((_, fs, _)) = guard.iter().find(|(id, _, _)| *id == mount_id) {
        return op(&**fs);
    }
    Err("mount not found")
}

/// Invoke a closure with the optional upper filesystem for `mount_id`.
/// The closure receives `Some(&dyn FileSystem)` when an upper was registered,
/// or `None` when there is no explicit upper. Returns `Err("mount not found")`
/// if the mount id is not registered.
pub fn with_upper<T>(mount_id: usize, op: impl FnOnce(Option<&dyn FileSystem>) -> Result<T, &'static str>) -> Result<T, &'static str> {
    let guard = OVERLAY_INSTANCES.lock();
    if let Some((_, _, upper)) = guard.iter().find(|(id, _, _)| *id == mount_id) {
        let opt = upper.as_ref().map(|b| &**b);
        return op(opt);
    }
    Err("mount not found")
}
