use super::*;
use crate::kernel::vfs_control;
use crate::kernel::{
    so_loader::SharedObjectLoader,
    symbol::{Symbol, SymbolTable},
};
use core::sync::atomic::{AtomicBool, AtomicU32};

fn make_test_so_image() -> alloc::vec::Vec<u8> {
    let mut img = alloc::vec![0u8; 0x4000];
    let name_off_in_strtab: u32 = 0;
    let name_bytes = b"foo\0";
    img[0x2000..0x2000 + name_bytes.len()].copy_from_slice(name_bytes);
    let sym_off = 0x3000usize;
    img[sym_off..sym_off + 4].copy_from_slice(&name_off_in_strtab.to_le_bytes());
    img[sym_off + 4] = 0u8;
    let sym_addr: u64 = 0x1000;
    img[sym_off + 8..sym_off + 16].copy_from_slice(&sym_addr.to_le_bytes());
    let sym_size: u64 = 8;
    img[sym_off + 16..sym_off + 24].copy_from_slice(&sym_size.to_le_bytes());
    img
}

#[test]
fn test_dlopen_dlsym_integration() {
    let _ = vfs_control::mount_ramfs(b"/lib").expect("mount ramfs");
    let img = make_test_so_image();
    let entries = [("/lib/libtest.so", img.as_slice())];
    let _ = vfs_control::load_initrd_entries(1, &entries).expect("load entries");

    let handle = dlopen(b"/lib/libtest.so\0".as_ptr(), 0);
    assert!(!handle.is_null());

    let sym_ptr = dlsym(handle, b"foo\0".as_ptr());
    assert!(!sym_ptr.is_null());
}

#[test]
fn test_dlsym_supports_global_and_object_handles() {
    let _ = vfs_control::mount_ramfs(b"/lib").expect("mount ramfs");
    let img = make_test_so_image();
    let entries = [("/lib/libglobal.so", img.as_slice())];
    let _ = vfs_control::load_initrd_entries(1, &entries).expect("load entries");

    let object_handle = dlopen(b"/lib/libglobal.so\0".as_ptr(), 0);
    assert!(!object_handle.is_null());
    assert!(!dlsym(object_handle, b"foo\0".as_ptr()).is_null());

    let global_handle = dlopen(core::ptr::null(), 0);
    assert!(!global_handle.is_null());
    assert!(dlsym(global_handle, b"foo\0".as_ptr()).is_null());

    let exported_handle = dlopen(b"/lib/libglobal.so\0".as_ptr(), 0x100);
    assert!(!exported_handle.is_null());
    assert!(!dlsym(global_handle, b"foo\0".as_ptr()).is_null());

    assert_eq!(dlclose(global_handle), 0);
    assert_eq!(dlclose(exported_handle), 0);
    assert_eq!(dlclose(object_handle), 0);
}

#[test]
fn test_dlclose_tracks_pending_fini_reports() {
    let _ = vfs_control::mount_ramfs(b"/lib").expect("mount ramfs");
    let img = make_test_so_image();
    let entries = [("/lib/libfini.so", img.as_slice())];
    let _ = vfs_control::load_initrd_entries(1, &entries).expect("load entries");

    let handle = dlopen(b"/lib/libfini.so\0".as_ptr(), 0);
    assert!(!handle.is_null());
    assert_eq!(dlclose(handle), 0);

    let _ = pending_shared_object_fini_count();
    let _ = drain_pending_shared_object_fini_reports();
}

#[test]
fn test_dlopen_noload_rejects_absent_object() {
    let handle = dlopen(b"/lib/libmissing.so\0".as_ptr(), 0x4);
    assert!(handle.is_null());
}

#[test]
fn test_dlopen_noload_global_promotes_existing_object() {
    let _ = vfs_control::mount_ramfs(b"/lib").expect("mount ramfs");
    let img = make_test_so_image();
    let entries = [("/lib/libpromote.so", img.as_slice())];
    let _ = vfs_control::load_initrd_entries(1, &entries).expect("load entries");

    let local_handle = dlopen(b"/lib/libpromote.so\0".as_ptr(), 0);
    assert!(!local_handle.is_null());

    let global_handle = dlopen(core::ptr::null(), 0);
    assert!(dlsym(global_handle, b"foo\0".as_ptr()).is_null());

    let promote_handle = dlopen(b"/lib/libpromote.so\0".as_ptr(), 0x4 | 0x100);
    assert!(!promote_handle.is_null());
    assert!(!dlsym(global_handle, b"foo\0".as_ptr()).is_null());

    assert_eq!(dlclose(global_handle), 0);
    assert_eq!(dlclose(promote_handle), 0);
    assert_eq!(dlclose(local_handle), 0);
}

#[test]
fn test_loader_object_versioned_lookup_prefers_matching_version() {
    let mut loader = SharedObjectLoader::new();
    loader.loaded.push(crate::kernel::so_loader::SharedObject {
        name: "libversioned.so".into(),
        base_addr: 0x1000,
        symbols: SymbolTable {
            symbols: alloc::vec![
                Symbol {
                    name: "foo".into(),
                    addr: 0x1111,
                    size: 8,
                    st_info: 0,
                    vers: Some(1),
                    vers_name: Some("VER_1".into()),
                },
                Symbol {
                    name: "foo".into(),
                    addr: 0x2222,
                    size: 8,
                    st_info: 0,
                    vers: Some(2),
                    vers_name: Some("VER_2".into()),
                },
            ],
        },
        refcount: AtomicU32::new(1),
        loaded_at: 0,
        global_visible: AtomicBool::new(false),
        nodelete: AtomicBool::new(false),
        runtime_hooks: crate::kernel::process::RuntimeLifecycleHooks::default(),
        dependencies: alloc::vec::Vec::new(),
    });

    let matched = loader
        .find_symbol_in_object_versioned("libversioned.so", "foo", Some("VER_2"))
        .expect("versioned symbol");
    assert_eq!(matched.addr, 0x2222);

    let fallback = loader
        .find_symbol_in_object_versioned("libversioned.so", "foo", Some("VER_MISSING"))
        .expect("fallback symbol");
    assert_eq!(fallback.addr, 0x1111);
}

#[test]
fn test_loader_nodelete_preserves_object_on_unload() {
    let mut loader = SharedObjectLoader::new();
    loader.loaded.push(crate::kernel::so_loader::SharedObject {
        name: "libnodelete.so".into(),
        base_addr: 0x2000,
        symbols: SymbolTable {
            symbols: alloc::vec::Vec::new(),
        },
        refcount: AtomicU32::new(1),
        loaded_at: 0,
        global_visible: AtomicBool::new(false),
        nodelete: AtomicBool::new(true),
        runtime_hooks: crate::kernel::process::RuntimeLifecycleHooks::default(),
        dependencies: alloc::vec::Vec::new(),
    });

    let report = loader
        .unload("libnodelete.so", None)
        .expect("unload report");
    assert!(!report.unloaded);
    assert!(loader.is_loaded("libnodelete.so"));
}
