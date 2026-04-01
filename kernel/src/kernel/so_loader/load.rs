use alloc::string::{String, ToString};
use core::sync::atomic::Ordering;

use super::{runtime_hooks::parse_needed_dependencies, runtime_hooks::parse_runtime_hooks};
use super::{SharedObject, SharedObjectLoader, FALLBACK_SYMTAB_COUNT};

impl SharedObjectLoader {
    pub fn load_needed(&mut self, needed: &[alloc::string::String]) {
        let mut visited = alloc::collections::BTreeSet::new();
        for lib in needed {
            self.load_recursive(lib, &mut visited);
        }
    }

    fn load_recursive(&mut self, lib: &str, visited: &mut alloc::collections::BTreeSet<String>) {
        if visited.contains(lib) {
            return;
        }
        visited.insert(lib.to_string());
        for so in &self.loaded {
            if so.name == lib {
                so.refcount.fetch_add(1, Ordering::Relaxed);
                return;
            }
        }
        let image = match self.load_image(lib) {
            Some(image) => image,
            None => {
                crate::klog_warn!("shared object load failed: {}", lib);
                return;
            }
        };
        let Some(dyn_sec) = super::super::elf_dynamic::DynamicSection::parse(&image, 0x800, 32)
        else {
            crate::klog_warn!("shared object dynamic section parse failed: {}", lib);
            return;
        };
        let strtab_off = dyn_sec.dt_strtab.unwrap_or(0) as usize;
        let symtab_off = dyn_sec.dt_symtab.unwrap_or(0) as usize;
        let sym_count = dyn_sec
            .dt_hash
            .and_then(|hash_off| {
                super::super::elf_dynamic::parse_sysv_hash_nchain(&image, hash_off as usize)
            })
            .or_else(|| {
                dyn_sec.dt_gnu_hash.and_then(|hash_off| {
                    super::super::elf_dynamic::parse_gnu_hash_symbol_count(
                        &image,
                        hash_off as usize,
                    )
                })
            })
            .unwrap_or(FALLBACK_SYMTAB_COUNT);
        let Some(mut symtab) = super::super::symbol::SymbolTable::parse(
            &image,
            strtab_off as u64,
            symtab_off as u64,
            sym_count,
            dyn_sec.dt_versym,
            dyn_sec.dt_verneed,
            dyn_sec.dt_verdef,
        ) else {
            return;
        };

        let soname = dyn_sec.dt_soname.and_then(|name_off| {
            let start = strtab_off.saturating_add(name_off as usize);
            if start >= image.len() {
                return None;
            }
            let mut end = start;
            while end < image.len() && image[end] != 0 {
                end += 1;
            }
            core::str::from_utf8(&image[start..end])
                .ok()
                .map(|s| s.to_string())
        });
        validate_soname(lib, soname.as_deref());

        let has_version_tables =
            dyn_sec.dt_versym.is_some() || dyn_sec.dt_verdef.is_some() || dyn_sec.dt_verneed.is_some();
        if has_version_tables && symtab.symbols.iter().all(|sym| sym.vers_name.is_none()) {
            crate::klog_warn!(
                "shared object version tables present but symbol version names unresolved: {}",
                lib
            );
        }

        let base_addr = super::SYNTHETIC_SO_BASE_START
            + (visited.len() as u64 * super::SYNTHETIC_SO_BASE_STRIDE);
        let runtime_hooks = parse_runtime_hooks(&image, &dyn_sec, base_addr).unwrap_or_default();
        let dependencies = parse_needed_dependencies(&image, &dyn_sec, strtab_off);
        for s in symtab.symbols.iter_mut() {
            s.addr = s.addr.wrapping_add(base_addr);
        }
        let now = crate::hal::cpu::rdtsc();
        self.loaded.push(SharedObject {
            name: lib.to_string(),
            base_addr,
            symbols: symtab,
            refcount: core::sync::atomic::AtomicU32::new(1),
            loaded_at: now,
            global_visible: core::sync::atomic::AtomicBool::new(false),
            nodelete: core::sync::atomic::AtomicBool::new(false),
            runtime_hooks,
            dependencies,
        });
        for dep in &dyn_sec.dt_needed {
            let name_offset = *dep as usize;
            let mut end = strtab_off + name_offset;
            while end < image.len() && image[end] != 0 {
                end += 1;
            }
            if let Ok(dep_name) = core::str::from_utf8(&image[strtab_off + name_offset..end]) {
                self.load_recursive(dep_name, visited);
            }
        }
    }

    fn load_image(&self, lib: &str) -> Option<alloc::vec::Vec<u8>> {
        #[cfg(feature = "vfs")]
        {
            use crate::interfaces::TaskId;
            use crate::kernel::vfs_control;
            use crate::modules::vfs::SeekFrom;
            let mut candidates = alloc::vec::Vec::new();
            for dir in &self.search_paths {
                let trimmed = dir.trim_end_matches('/');
                if !trimmed.is_empty() {
                    candidates.push(alloc::format!("{}/{}", trimmed, lib));
                }
            }
            candidates.push(alloc::format!("/lib/{}", lib));
            candidates.push(alloc::format!("/usr/lib/{}", lib));
            candidates.push(lib.to_string());
            let tid = unsafe {
                crate::kernel::cpu_local::CpuLocal::try_get()
                    .map(|cpu| TaskId(cpu.current_task.load(Ordering::Relaxed)))
                    .unwrap_or(TaskId(0))
            };

            for path in &candidates {
                if let Ok(mut file) = vfs_control::ramfs_open_file(1, path.as_str(), tid) {
                    let mut out = alloc::vec::Vec::new();
                    let _ = file.seek(SeekFrom::Start(0));
                    let mut buf = [0u8; crate::interfaces::memory::PAGE_SIZE_4K];
                    loop {
                        match file.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => out.extend_from_slice(&buf[..n]),
                            Err(_) => break,
                        }
                    }
                    if !out.is_empty() {
                        return Some(out);
                    }
                }
            }
        }

        let msg = alloc::format!("failed to load shared object: {}", lib);
        crate::klog_warn!("{}", msg);
        None
    }
}

fn normalize_object_basename(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

fn validate_soname(requested_name: &str, soname: Option<&str>) {
    let Some(soname_name) = soname else {
        return;
    };
    if soname_name.is_empty() {
        crate::klog_warn!("shared object has empty DT_SONAME: {}", requested_name);
        return;
    }

    let requested_base = normalize_object_basename(requested_name);
    if requested_base != soname_name {
        crate::klog_warn!(
            "shared object SONAME mismatch: requested={} soname={}",
            requested_base,
            soname_name
        );
    }
}
