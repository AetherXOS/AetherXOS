//! Shared object loader for DT_NEEDED dependencies

use alloc::string::ToString;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::interfaces::task::ProcessId;

mod load;
mod runtime_hooks;
mod unload;

pub(crate) const SYNTHETIC_SO_BASE_START: u64 = 0x8000_0000;
pub(crate) const SYNTHETIC_SO_BASE_STRIDE: u64 = 0x0100_0000;
pub(crate) const FALLBACK_SYMTAB_COUNT: usize = 256;

#[derive(Debug, Clone)]
pub struct SharedObjectUnloadReport {
    pub name: alloc::string::String,
    pub owner_process_id: Option<ProcessId>,
    pub fini_calls: alloc::vec::Vec<u64>,
    pub unloaded: bool,
    pub dependency_unloads: usize,
}

pub struct SharedObject {
    pub name: alloc::string::String,
    pub base_addr: u64,
    pub symbols: super::symbol::SymbolTable,
    pub refcount: core::sync::atomic::AtomicU32,
    pub loaded_at: u64,
    pub global_visible: AtomicBool,
    pub nodelete: AtomicBool,
    pub runtime_hooks: crate::kernel::process::RuntimeLifecycleHooks,
    pub dependencies: alloc::vec::Vec<alloc::string::String>,
}

pub struct SharedObjectLoader {
    pub loaded: alloc::vec::Vec<SharedObject>,
    pub mutex: spin::Mutex<()>,
    pub plt_slots: alloc::vec::Vec<(u64, alloc::string::String, bool, u64)>,
    pub search_paths: alloc::vec::Vec<alloc::string::String>,
    pub pending_fini: alloc::vec::Vec<SharedObjectUnloadReport>,
}

impl SharedObjectLoader {
    pub fn new() -> Self {
        Self::with_search_paths(&[])
    }

    pub fn with_search_paths(search_paths: &[alloc::string::String]) -> Self {
        let mut deduped = alloc::vec::Vec::new();
        for path in search_paths {
            if path.is_empty()
                || deduped
                    .iter()
                    .any(|existing: &alloc::string::String| existing == path)
            {
                continue;
            }
            deduped.push(path.clone());
        }
        Self {
            loaded: alloc::vec::Vec::new(),
            mutex: spin::Mutex::new(()),
            plt_slots: alloc::vec::Vec::new(),
            search_paths: deduped,
            pending_fini: alloc::vec::Vec::new(),
        }
    }

    pub fn find_symbol(&self, name: &str) -> Option<super::symbol::Symbol> {
        let _g = self.mutex.lock();
        for so in &self.loaded {
            if !so.global_visible.load(Ordering::Relaxed) {
                continue;
            }
            if let Some(sym) = so.symbols.find_by_name(name) {
                return Some(sym.clone());
            }
        }
        None
    }

    pub fn find_symbol_in_object(
        &self,
        object_name: &str,
        symbol_name: &str,
    ) -> Option<super::symbol::Symbol> {
        let _g = self.mutex.lock();
        self.loaded
            .iter()
            .find(|so| so.name == object_name)
            .and_then(|so| so.symbols.find_by_name(symbol_name).cloned())
    }

    pub fn find_symbol_in_object_versioned(
        &self,
        object_name: &str,
        symbol_name: &str,
        version: Option<&str>,
    ) -> Option<super::symbol::Symbol> {
        let _g = self.mutex.lock();
        let so = self.loaded.iter().find(|so| so.name == object_name)?;
        if let Some(version_name) = version {
            return so
                .symbols
                .symbols
                .iter()
                .find(|s| s.name == symbol_name && s.vers_name.as_deref() == Some(version_name))
                .cloned();
        }
        so.symbols.find_by_name(symbol_name).cloned()
    }

    pub fn is_loaded(&self, object_name: &str) -> bool {
        let _g = self.mutex.lock();
        self.loaded.iter().any(|so| so.name == object_name)
    }

    pub fn promote_global_visibility(&mut self, object_name: &str) -> bool {
        let _g = self.mutex.lock();
        if let Some(so) = self.loaded.iter().find(|so| so.name == object_name) {
            so.global_visible.store(true, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn promote_global_visibility_recursive(&mut self, object_name: &str) -> usize {
        let _g = self.mutex.lock();
        let mut promoted = 0usize;
        let mut stack = alloc::vec![object_name.to_string()];
        let mut visited = alloc::collections::BTreeSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if let Some(so) = self.loaded.iter().find(|so| so.name == current) {
                if !so.global_visible.swap(true, Ordering::Relaxed) {
                    promoted = promoted.saturating_add(1);
                }
                for dependency in &so.dependencies {
                    stack.push(dependency.clone());
                }
            }
        }

        promoted
    }

    pub fn mark_nodelete_recursive(&mut self, object_name: &str) -> usize {
        let _g = self.mutex.lock();
        let mut marked = 0usize;
        let mut stack = alloc::vec![object_name.to_string()];
        let mut visited = alloc::collections::BTreeSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }

            if let Some(so) = self.loaded.iter().find(|so| so.name == current) {
                if !so.nodelete.swap(true, Ordering::Relaxed) {
                    marked = marked.saturating_add(1);
                }
                for dependency in &so.dependencies {
                    stack.push(dependency.clone());
                }
            }
        }

        marked
    }

    pub fn object_runtime_hooks(
        &self,
        object_name: &str,
    ) -> Option<crate::kernel::process::RuntimeLifecycleHooks> {
        let _g = self.mutex.lock();
        self.loaded
            .iter()
            .find(|so| so.name == object_name)
            .map(|so| so.runtime_hooks.clone())
    }

    pub fn pending_fini_count(&self) -> usize {
        let _g = self.mutex.lock();
        self.pending_fini.len()
    }

    pub fn drain_pending_fini_reports(&mut self) -> alloc::vec::Vec<SharedObjectUnloadReport> {
        let _g = self.mutex.lock();
        core::mem::take(&mut self.pending_fini)
    }

    pub fn drain_pending_fini_reports_for_process(
        &mut self,
        process_id: ProcessId,
    ) -> alloc::vec::Vec<SharedObjectUnloadReport> {
        let _g = self.mutex.lock();
        let mut drained = alloc::vec::Vec::new();
        let mut retained = alloc::vec::Vec::with_capacity(self.pending_fini.len());
        for report in self.pending_fini.drain(..) {
            if report.owner_process_id == Some(process_id) {
                drained.push(report);
            } else {
                retained.push(report);
            }
        }
        self.pending_fini = retained;
        drained
    }

    pub fn find_symbol_versioned(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Option<super::symbol::Symbol> {
        let _g = self.mutex.lock();
        if let Some(v) = version {
            for so in &self.loaded {
                if !so.global_visible.load(Ordering::Relaxed) {
                    continue;
                }
                if let Some(sym) = so
                    .symbols
                    .symbols
                    .iter()
                    .find(|s| s.name == name && s.vers_name.as_deref() == Some(v))
                {
                    return Some(sym.clone());
                }
            }
            return None;
        }
        for so in &self.loaded {
            if !so.global_visible.load(Ordering::Relaxed) {
                continue;
            }
            if let Some(sym) = so.symbols.find_by_name(name) {
                return Some(sym.clone());
            }
        }
        None
    }

    pub fn register_plt_slot(
        &mut self,
        vaddr: u64,
        name: &str,
        is_ifunc: bool,
        resolver_vaddr: u64,
    ) {
        let _g = self.mutex.lock();
        self.plt_slots
            .push((vaddr, name.to_string(), is_ifunc, resolver_vaddr));
    }
}

#[cfg(all(test, any()))]
#[path = "unload_tests.rs"]
mod unload_tests;
