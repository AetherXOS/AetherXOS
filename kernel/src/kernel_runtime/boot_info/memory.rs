use super::{BootInfo, MAX_USABLE_REGIONS, MemRegion};

pub(super) fn collect_hhdm_offset(info: &mut BootInfo) {
    #[cfg(target_arch = "x86_64")]
    {
        info.hhdm_offset = aethercore::hal::hhdm_offset().unwrap_or(0);
    }
}

pub(super) fn collect_memory_map(info: &mut BootInfo) {
    #[cfg(target_arch = "x86_64")]
    {
        use limine::MemoryMapEntryType;

        if let Some(mmap) = aethercore::hal::mem_map() {
            for entry_ptr in mmap.memmap() {
                let entry_raw = entry_ptr.as_ptr();
                if entry_raw.is_null() {
                    continue;
                }
                let entry = unsafe { &*entry_raw };

                info.map_entry_count += 1;
                info.total_map_bytes = info.total_map_bytes.saturating_add(entry.len);

                if entry.typ == MemoryMapEntryType::Usable {
                    push_usable_region(
                        info,
                        MemRegion {
                            base: entry.base,
                            len: entry.len,
                        },
                    );
                }
            }

            sort_usable_regions(info);
        }
    }
}

fn push_usable_region(info: &mut BootInfo, region: MemRegion) {
    info.total_usable_bytes = info.total_usable_bytes.saturating_add(region.len);
    if region.len > info.largest_region.len {
        info.largest_region = region;
    }

    if info.usable_region_count < MAX_USABLE_REGIONS {
        info.usable_regions[info.usable_region_count] = region;
        info.usable_region_count += 1;
    }
}

fn sort_usable_regions(info: &mut BootInfo) {
    let count = info.usable_region_count;
    for i in 1..count {
        let mut j = i;
        while j > 0 && info.usable_regions[j - 1].base > info.usable_regions[j].base {
            info.usable_regions.swap(j - 1, j);
            j -= 1;
        }
    }
}
