use super::*;

#[test_case]
fn unload_fails_when_plt_references_symbol() {
    let mut loader = SharedObjectLoader::new();

    let sym = super::super::symbol::Symbol {
        name: alloc::string::String::from("foo"),
        addr: 0x1000,
        size: 0,
        st_info: 0,
        vers: None,
        vers_name: None,
    };
    let symtab = super::super::symbol::SymbolTable {
        symbols: alloc::vec::Vec::from([sym.clone()]),
    };
    let so = SharedObject {
        name: alloc::string::String::from("libx.so"),
        base_addr: 0x8000,
        symbols: symtab,
        refcount: core::sync::atomic::AtomicU32::new(1),
        loaded_at: 0,
        global_visible: core::sync::atomic::AtomicBool::new(false),
        nodelete: core::sync::atomic::AtomicBool::new(false),
        runtime_hooks: crate::kernel::process::RuntimeLifecycleHooks::default(),
        dependencies: alloc::vec::Vec::new(),
    };
    loader.loaded.push(so);

    loader.register_plt_slot(0x2000, "foo", false, 0);
    assert!(loader.unload("libx.so", None).is_err());

    loader.plt_slots.clear();
    assert!(loader.unload("libx.so", None).is_ok());
}

