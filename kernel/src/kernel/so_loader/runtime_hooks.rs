use alloc::string::ToString;
use xmas_elf::program::Type;
use xmas_elf::ElfFile;

pub(super) fn parse_needed_dependencies(
    image: &[u8],
    dynamic: &super::super::elf_dynamic::DynamicSection,
    strtab_off: usize,
) -> alloc::vec::Vec<alloc::string::String> {
    let mut dependencies = alloc::vec::Vec::new();
    for dep in &dynamic.dt_needed {
        let name_offset = *dep as usize;
        let start = strtab_off.saturating_add(name_offset);
        if start >= image.len() {
            continue;
        }
        let mut end = start;
        while end < image.len() && image[end] != 0 {
            end += 1;
        }
        if let Ok(dep_name) = core::str::from_utf8(&image[start..end]) {
            if !dep_name.is_empty() && !dependencies.iter().any(|existing| existing == dep_name) {
                dependencies.push(dep_name.to_string());
            }
        }
    }
    dependencies
}

pub(super) fn parse_runtime_hooks(
    image: &[u8],
    dynamic: &super::super::elf_dynamic::DynamicSection,
    base_addr: u64,
) -> Option<crate::kernel::process::RuntimeLifecycleHooks> {
    let elf = ElfFile::new(image).ok()?;
    let vaddr_to_offset = |vaddr: u64| -> Option<usize> {
        for ph in elf.program_iter() {
            if let Ok(Type::Load) = ph.get_type() {
                let va = ph.virtual_addr();
                let memsz = ph.mem_size();
                if vaddr >= va && vaddr < va + memsz {
                    let off = ph.offset() + (vaddr - va);
                    return usize::try_from(off).ok();
                }
            }
        }
        None
    };
    let runtime_addr = |raw: u64| raw.wrapping_add(base_addr);
    let read_pointer_array = |array_vaddr: u64, array_size: u64| -> alloc::vec::Vec<u64> {
        let mut out = alloc::vec::Vec::new();
        let Some(array_off) = vaddr_to_offset(array_vaddr) else {
            return out;
        };
        let Some(entries) = usize::try_from(array_size / 8).ok() else {
            return out;
        };
        for idx in 0..entries {
            let off = array_off + idx * 8;
            let Some(bytes) = image.get(off..off + 8) else {
                break;
            };
            let raw = u64::from_le_bytes(bytes.try_into().ok().unwrap_or([0; 8]));
            if raw != 0 {
                out.push(runtime_addr(raw));
            }
        }
        out
    };

    Some(crate::kernel::process::RuntimeLifecycleHooks {
        preinit_array: match (dynamic.dt_preinit_array, dynamic.dt_preinit_arraysz) {
            (Some(vaddr), Some(size)) => read_pointer_array(vaddr, size),
            _ => alloc::vec::Vec::new(),
        },
        init: dynamic.dt_init.map(runtime_addr),
        init_array: match (dynamic.dt_init_array, dynamic.dt_init_arraysz) {
            (Some(vaddr), Some(size)) => read_pointer_array(vaddr, size),
            _ => alloc::vec::Vec::new(),
        },
        deferred_fini: alloc::vec::Vec::new(),
        fini_array: match (dynamic.dt_fini_array, dynamic.dt_fini_arraysz) {
            (Some(vaddr), Some(size)) => read_pointer_array(vaddr, size),
            _ => alloc::vec::Vec::new(),
        },
        fini: dynamic.dt_fini.map(runtime_addr),
    })
}
