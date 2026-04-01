//! Heap initialisation for each supported platform.
//!
//! ## x86_64
//! Scans the Limine memory map and initialises the allocator with the **largest**
//! contiguous usable region.  If the kernel heap size config fits into multiple
//! regions the largest is preferred for better allocator performance.
//!
//! ## AArch64
//! Falls back to a 32 MiB static heap (enough for kernel boot) until a proper
//! DTB / UEFI memory map parser hands us a dynamic range.

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use hypercore::interfaces::memory::HeapAllocator;

const BYTES_PER_MIB: usize = 1024 * 1024;
static PENDING_HEAP_PHYS_ADDR: AtomicUsize = AtomicUsize::new(0);
static PENDING_HEAP_VIRT_ADDR: AtomicUsize = AtomicUsize::new(0);
static PENDING_HEAP_ACTUAL_SIZE: AtomicUsize = AtomicUsize::new(0);
static PENDING_HEAP_BEST_LEN: AtomicUsize = AtomicUsize::new(0);
static PENDING_COMPACTION_BASE: AtomicUsize = AtomicUsize::new(0);
static PENDING_COMPACTION_PAGES: AtomicUsize = AtomicUsize::new(0);
static PENDING_HEAP_FINALIZE: AtomicBool = AtomicBool::new(false);

pub(super) fn init_heap(allocator: &hypercore::modules::allocators::selector::ActiveHeapAllocator) {
    use hypercore::generated_consts::MEM_HEAP_SIZE_MB;

    #[cfg(target_arch = "x86_64")]
    use limine::MemoryMapEntryType;

    #[allow(unused_variables)]
    let heap_size = MEM_HEAP_SIZE_MB.saturating_mul(BYTES_PER_MIB);

    // ── x86_64: scan Limine memory map ───────────────────────────────────────
    #[cfg(target_arch = "x86_64")]
    {
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] heap init entry\n");
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] heap init hhdm query\n");
        let hhdm = hypercore::hal::x86_64::hhdm_offset().unwrap_or(0);
        let _ = hhdm;
        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] heap init hhdm ready\n");

        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] heap init memmap query\n");
        if let Some(mmap) = hypercore::hal::x86_64::mem_map() {
            hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] heap init memmap ready\n");
            // Pick the largest usable region ≥ heap_size.
            let mut best_base: u64 = 0;
            let mut best_len: u64 = 0;

            for entry_ptr in mmap.memmap() {
                let entry_raw = entry_ptr.as_ptr();
                if entry_raw.is_null() {
                    continue;
                }
                let entry = unsafe { &*entry_raw };

                if entry.typ == MemoryMapEntryType::Usable
                    && entry.len >= heap_size as u64
                    && entry.len > best_len
                {
                    best_base = entry.base;
                    best_len = entry.len;
                }
            }

            hypercore::hal::x86_64::serial::write_raw(
                "[EARLY SERIAL] heap init memmap scan complete\n",
            );

            if best_base != 0 {
                let phys_addr = best_base;
                let virt_addr = phys_addr + hhdm;
                // Cap the region at the configured heap size so we don't over-commit.
                let actual_size = (best_len as usize).min(heap_size);
                #[cfg(target_arch = "x86_64")]
                hypercore::hal::x86_64::serial::write_raw(
                    "[EARLY SERIAL] heap allocator init begin\n",
                );
                allocator.init(virt_addr as usize, actual_size);
                #[cfg(target_arch = "x86_64")]
                hypercore::hal::x86_64::serial::write_raw(
                    "[EARLY SERIAL] heap allocator init complete\n",
                );
                PENDING_HEAP_PHYS_ADDR.store(phys_addr as usize, Ordering::Relaxed);
                PENDING_HEAP_VIRT_ADDR.store(virt_addr as usize, Ordering::Relaxed);
                PENDING_HEAP_ACTUAL_SIZE.store(actual_size, Ordering::Relaxed);
                PENDING_HEAP_BEST_LEN.store(best_len as usize, Ordering::Relaxed);

                // Register the remainder of the region as compaction candidates
                // so the buddy allocator can reclaim them later.
                let remainder = best_len as usize - actual_size;
                if remainder >= 4096 {
                    PENDING_COMPACTION_BASE
                        .store((phys_addr as usize) + actual_size, Ordering::Relaxed);
                    PENDING_COMPACTION_PAGES.store(remainder / 4096, Ordering::Relaxed);
                }
                PENDING_HEAP_FINALIZE.store(true, Ordering::Relaxed);
                return;
            }
        }

        hypercore::hal::x86_64::serial::write_raw("[EARLY SERIAL] heap init no usable region\n");
        hypercore::klog_error!("No usable memory region ≥ {} MiB found!", MEM_HEAP_SIZE_MB);
        hypercore::kernel::fatal_halt("out of memory during heap init");
    }

    // ── AArch64: larger static heap with DTB fallback notice ──────────────────
    #[cfg(target_arch = "aarch64")]
    {
        // 32 MiB static region.  In production this should be replaced with
        // regions discovered from the DTB `memory` node or UEFI memory map.
        const AARCH64_HEAP_SIZE: usize = 32 * BYTES_PER_MIB;
        static AARCH64_HEAP: spin::Mutex<[u8; AARCH64_HEAP_SIZE]> =
            spin::Mutex::new([0u8; AARCH64_HEAP_SIZE]);

        let ptr = {
            let mut guard = AARCH64_HEAP.lock();
            let p = guard.as_mut_ptr();
            drop(guard);
            p
        };

        hypercore::klog_info!(
            "Heap (AArch64 static fallback): ptr={:#x} size={} MiB",
            ptr as usize,
            AARCH64_HEAP_SIZE / BYTES_PER_MIB
        );
        allocator.init(ptr as usize, AARCH64_HEAP_SIZE);

        // If a DTB gives us additional memory, it will be hotplugged later via
        // hypercore::modules::allocators::advanced::hotplug_add_memory().
        if let Some(dtb_phys) = hypercore::hal::dtb_addr() {
            hypercore::klog_info!(
                "Heap: DTB at {:#x} — dynamic memory regions should be added via hotplug",
                dtb_phys
            );
        }
    }
}

pub(super) fn finalize_heap_bootstrap() {
    if !PENDING_HEAP_FINALIZE.swap(false, Ordering::Relaxed) {
        return;
    }

    let virt_addr = PENDING_HEAP_VIRT_ADDR.load(Ordering::Relaxed);
    let phys_addr = PENDING_HEAP_PHYS_ADDR.load(Ordering::Relaxed);
    let actual_size = PENDING_HEAP_ACTUAL_SIZE.load(Ordering::Relaxed);
    let best_len = PENDING_HEAP_BEST_LEN.load(Ordering::Relaxed);
    hypercore::klog_info!(
        "Heap: virt={:#x} phys={:#x} size={} MiB ({} MiB available)",
        virt_addr,
        phys_addr,
        actual_size / BYTES_PER_MIB,
        best_len / BYTES_PER_MIB
    );

    let compaction_pages = PENDING_COMPACTION_PAGES.swap(0, Ordering::Relaxed);
    if compaction_pages != 0 {
        let compaction_base = PENDING_COMPACTION_BASE.swap(0, Ordering::Relaxed);
        hypercore::modules::allocators::advanced::register_compaction_candidate(
            compaction_base,
            compaction_pages,
        );
        hypercore::klog_info!(
            "Heap: {} MiB remainder registered for memory compaction",
            (compaction_pages * 4096) / BYTES_PER_MIB
        );
    }
}
