use alloc::string::ToString;
use core::sync::atomic::Ordering;

use crate::interfaces::task::ProcessId;

use super::{SharedObjectLoader, SharedObjectUnloadReport};

impl SharedObjectLoader {
    pub fn unload(
        &mut self,
        name: &str,
        owner_process_id: Option<ProcessId>,
    ) -> Result<SharedObjectUnloadReport, &'static str> {
        self.unload_locked(name, owner_process_id)
    }

    fn unload_locked(
        &mut self,
        name: &str,
        owner_process_id: Option<ProcessId>,
    ) -> Result<SharedObjectUnloadReport, &'static str> {
        if let Some(pos) = self.loaded.iter().position(|so| so.name == name) {
            let so = &self.loaded[pos];
            let prev = so.refcount.fetch_sub(1, Ordering::Relaxed);
            if prev > 1 {
                return Ok(SharedObjectUnloadReport {
                    name: name.to_string(),
                    owner_process_id,
                    fini_calls: alloc::vec::Vec::new(),
                    unloaded: false,
                    dependency_unloads: 0,
                });
            }

            if so.nodelete.load(Ordering::Relaxed) {
                so.refcount.store(0, Ordering::Relaxed);
                return Ok(SharedObjectUnloadReport {
                    name: name.to_string(),
                    owner_process_id,
                    fini_calls: alloc::vec::Vec::new(),
                    unloaded: false,
                    dependency_unloads: 0,
                });
            }

            for (vaddr, sname, _ifunc, _resolver) in &self.plt_slots {
                if so.symbols.find_by_name(sname).is_some() {
                    so.refcount.fetch_add(1, Ordering::Relaxed);
                    crate::klog_warn!(
                        "refusing to unload {}: PLT slot at {:#x} references symbol {}",
                        name,
                        vaddr,
                        sname
                    );
                    return Err("module in use by PLT slots");
                }
            }

            let fini_calls = so.runtime_hooks.ordered_fini_calls();
            let dependencies = so.dependencies.clone();
            let report = SharedObjectUnloadReport {
                name: name.to_string(),
                owner_process_id,
                fini_calls,
                unloaded: true,
                dependency_unloads: 0,
            };
            let _ = self.loaded.remove(pos);
            if !report.fini_calls.is_empty() {
                self.pending_fini.push(report.clone());
            }
            let mut dependency_unloads = 0usize;
            for dependency in dependencies {
                if let Ok(dep_report) = self.unload_locked(&dependency, owner_process_id) {
                    if dep_report.unloaded {
                        dependency_unloads = dependency_unloads.saturating_add(1);
                    }
                    dependency_unloads =
                        dependency_unloads.saturating_add(dep_report.dependency_unloads);
                }
            }
            return Ok(SharedObjectUnloadReport {
                dependency_unloads,
                ..report
            });
        }
        Err("module not found")
    }
}
