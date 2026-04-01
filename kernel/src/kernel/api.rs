//! Minimal POSIX dynamic linking API stubs (dlopen, dlsym, dlclose)

use crate::kernel::sync::IrqSafeMutex;
use crate::{klog_info, klog_warn};
use alloc::string::ToString;
use lazy_static::lazy_static;

const RTLD_LOCAL: i32 = 0;
const RTLD_NOW: i32 = 0x2;
const RTLD_NOLOAD: i32 = 0x4;
const RTLD_GLOBAL: i32 = 0x100;
const RTLD_NODELETE: i32 = 0x1000;

const RTLD_DEFAULT_HANDLE: *mut u8 = usize::MAX as *mut u8;
const RTLD_NEXT_HANDLE: *mut u8 = (usize::MAX - 1) as *mut u8;

#[derive(Debug, Clone)]
enum DlopenScope {
    Global,
    Object(alloc::string::String),
}

#[derive(Debug, Clone)]
struct DlopenHandle {
    scope: DlopenScope,
    flags: i32,
}

#[inline(always)]
fn wants_noload(flags: i32) -> bool {
    (flags & RTLD_NOLOAD) != 0
}

#[inline(always)]
fn prefers_global_scope(flags: i32) -> bool {
    (flags & RTLD_GLOBAL) != 0
}

#[inline(always)]
fn wants_nodelete(flags: i32) -> bool {
    (flags & RTLD_NODELETE) != 0
}

fn split_versioned_symbol_name<'a>(name: &'a str) -> (&'a str, Option<&'a str>) {
    if let Some((base, version)) = name.split_once("@@") {
        (base, Some(version))
    } else if let Some((base, version)) = name.split_once('@') {
        (base, Some(version))
    } else {
        (name, None)
    }
}

lazy_static! {
    static ref SHARED_LOADER: IrqSafeMutex<super::so_loader::SharedObjectLoader> =
        IrqSafeMutex::new(super::so_loader::SharedObjectLoader::new());
}

pub fn pending_shared_object_fini_count() -> usize {
    SHARED_LOADER.lock().pending_fini_count()
}

pub fn drain_pending_shared_object_fini_reports(
) -> alloc::vec::Vec<super::so_loader::SharedObjectUnloadReport> {
    SHARED_LOADER.lock().drain_pending_fini_reports()
}

pub fn drain_pending_shared_object_fini_reports_for_process(
    process_id: crate::interfaces::task::ProcessId,
) -> alloc::vec::Vec<super::so_loader::SharedObjectUnloadReport> {
    SHARED_LOADER
        .lock()
        .drain_pending_fini_reports_for_process(process_id)
}

pub fn dlopen(_filename: *const u8, _flags: i32) -> *mut u8 {
    let _ = RTLD_NOW;
    let _ = RTLD_LOCAL;
    if _filename.is_null() {
        let boxed = alloc::boxed::Box::new(DlopenHandle {
            scope: DlopenScope::Global,
            flags: _flags,
        });
        return alloc::boxed::Box::into_raw(boxed) as *mut u8;
    }

    let mut len = 0;
    unsafe {
        while *_filename.add(len) != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(_filename, len);
        if let Ok(name) = core::str::from_utf8(slice) {
            let base = name.rsplit('/').next().unwrap_or(name);
            let mut loader = SHARED_LOADER.lock();
            if wants_noload(_flags) {
                if !loader.is_loaded(base) {
                    return core::ptr::null_mut();
                }
            } else {
                loader.load_needed(&[base.to_string()]);
            }
            if !loader.is_loaded(base) {
                return core::ptr::null_mut();
            }
            if prefers_global_scope(_flags) {
                let promoted = loader.promote_global_visibility_recursive(base);
                klog_info!("dlopen: promoted {} global objects for {}", promoted, base);
            }
            if wants_nodelete(_flags) {
                let marked = loader.mark_nodelete_recursive(base);
                klog_info!("dlopen: marked {} nodelete objects for {}", marked, base);
            }
            let boxed = alloc::boxed::Box::new(DlopenHandle {
                scope: DlopenScope::Object(alloc::string::String::from(base)),
                flags: _flags,
            });
            alloc::boxed::Box::into_raw(boxed) as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }
}

pub fn dlsym(_handle: *mut u8, _symbol: *const u8) -> *mut u8 {
    if _symbol.is_null() {
        return core::ptr::null_mut();
    }

    let mut len = 0;
    unsafe {
        while *_symbol.add(len) != 0 {
            len += 1;
        }
        let slice = core::slice::from_raw_parts(_symbol, len);
        if let Ok(name) = core::str::from_utf8(slice) {
            let (base_name, version) = split_versioned_symbol_name(name);
            let loader = SHARED_LOADER.lock();
            if _handle.is_null() || _handle == RTLD_DEFAULT_HANDLE || _handle == RTLD_NEXT_HANDLE {
                return loader
                    .find_symbol_versioned(base_name, version)
                    .map(|sym| sym.addr as *mut u8)
                    .unwrap_or(core::ptr::null_mut());
            }

            let handle = &*(_handle as *mut DlopenHandle);
            let _flags = handle.flags;
            match &handle.scope {
                DlopenScope::Global => loader
                    .find_symbol_versioned(base_name, version)
                    .map(|sym| sym.addr as *mut u8)
                    .unwrap_or(core::ptr::null_mut()),
                DlopenScope::Object(object_name) => {
                    let object_sym =
                        loader.find_symbol_in_object_versioned(object_name, base_name, version);
                    let resolved = if prefers_global_scope(_flags) {
                        object_sym.or_else(|| loader.find_symbol_versioned(base_name, version))
                    } else {
                        object_sym
                    };
                    resolved
                        .map(|sym| sym.addr as *mut u8)
                        .unwrap_or(core::ptr::null_mut())
                }
            }
        } else {
            core::ptr::null_mut()
        }
    }
}

pub fn dlclose(_handle: *mut u8) -> i32 {
    if _handle.is_null() {
        return 0;
    }
    unsafe {
        let boxed: alloc::boxed::Box<DlopenHandle> =
            alloc::boxed::Box::from_raw(_handle as *mut DlopenHandle);
        let scope = boxed.scope.clone();
        drop(boxed);

        match scope {
            DlopenScope::Global => 0,
            DlopenScope::Object(name) => {
                let owner_process_id = crate::kernel::launch::current_process_arc().map(|p| p.id);
                match SHARED_LOADER.lock().unload(&name, owner_process_id) {
                    Ok(report) => {
                        if !report.fini_calls.is_empty() {
                            klog_info!(
                            "dlclose: {} fini hooks pending count={} owner_pid={:?} dependency_unloads={}",
                            report.name,
                            report.fini_calls.len(),
                            report.owner_process_id,
                            report.dependency_unloads
                        );
                        }
                        if report.unloaded {
                            0
                        } else {
                            0
                        }
                    }
                    Err(e) => {
                        klog_warn!("dlclose: failed to unload {}: {}", name, e);
                        -1
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "vfs", any()))]
#[path = "api/tests.rs"]
mod tests;
