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

use aethercore::hal::Hal;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const BYTES_PER_MIB: usize = 1024 * 1024;
static PENDING_HEAP_PHYS_ADDR: AtomicUsize = AtomicUsize::new(0);
static PENDING_HEAP_VIRT_ADDR: AtomicUsize = AtomicUsize::new(0);
static PENDING_HEAP_ACTUAL_SIZE: AtomicUsize = AtomicUsize::new(0);
static PENDING_HEAP_BEST_LEN: AtomicUsize = AtomicUsize::new(0);
static PENDING_COMPACTION_BASE: AtomicUsize = AtomicUsize::new(0);
static PENDING_COMPACTION_PAGES: AtomicUsize = AtomicUsize::new(0);
static PENDING_HEAP_FINALIZE: AtomicBool = AtomicBool::new(false);
static HEAP_READY: AtomicBool = AtomicBool::new(false);

pub(super) fn init_heap(
    allocator: &aethercore::modules::allocators::selector::ActiveHeapAllocator,
) {
    use aethercore::generated_consts::MEM_HEAP_SIZE_MB;
    use aethercore::interfaces::memory::HeapAllocator;

    #[cfg(target_arch = "x86_64")]
    use limine::MemoryMapEntryType;

    #[allow(unused_variables)]
    let heap_size = MEM_HEAP_SIZE_MB.saturating_mul(BYTES_PER_MIB);

    // ── x86_64: scan Limine memory map ───────────────────────────────────────
    #[cfg(target_arch = "x86_64")]
    {
        Hal::serial_write_raw("[EARLY SERIAL] heap::init_heap x86_64 start\n");
        Hal::serial_write_raw("[EARLY SERIAL] heap::init_heap before hhdm\n");
        let hhdm = aethercore::hal::hhdm_offset().unwrap_or(0);
        let _ = hhdm;

        Hal::serial_write_raw("[EARLY SERIAL] heap::init_heap before mem_map\n");
        match aethercore::hal::mem_map() {
            Some(mmap) => {
                Hal::serial_write_raw("[EARLY SERIAL] heap::init_heap mem_map ready\n");
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

                if best_base != 0 {
                    let phys_addr = best_base;
                    let virt_addr = phys_addr + hhdm;
                    // Cap the region at the configured heap size so we don't over-commit.
                    let actual_size = (best_len as usize).min(heap_size);
                    Hal::serial_write_raw("[EARLY SERIAL] before allocator.init\n");
                    // SAFETY: This is only called once during early boot before multi-threading.
                    // The allocator uses atomics internally, so mutable access via const reference is safe.
                    unsafe {
                        let allocator_mut = allocator as *const _
                            as *mut aethercore::modules::allocators::selector::ActiveHeapAllocator;
                        (*allocator_mut).init(virt_addr as usize, actual_size);
                    }
                    Hal::serial_write_raw("[EARLY SERIAL] after allocator.init\n");
                    Hal::serial_write_raw("[EARLY SERIAL] heap::init_heap end (x86_64 success)\n");
                    HEAP_READY.store(true, Ordering::Release);
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

                Hal::serial_write_raw("[EARLY SERIAL] heap::init_heap end (ERROR: no usable region)\n");
                aethercore::kernel::fatal_halt("out of memory during heap init");
            }
            None => {
                Hal::serial_write_raw("[EARLY SERIAL] heap::init_heap end (ERROR: no memmap)\n");
                aethercore::kernel::fatal_halt("memory map unavailable");
            }
        }
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

        Hal::serial_write_raw("[EARLY SERIAL] before aarch64 allocator.init\n");
        allocator.init(ptr as usize, AARCH64_HEAP_SIZE);
        Hal::serial_write_raw("[EARLY SERIAL] after aarch64 allocator.init\n");
        Hal::serial_write_raw("[EARLY SERIAL] heap::init_heap end (aarch64 static)\n");
        HEAP_READY.store(true, Ordering::Release);

        // If a DTB gives us additional memory, it will be hotplugged later via
        // aethercore::modules::allocators::advanced::hotplug_add_memory().
        if let Some(dtb_phys) = aethercore::hal::dtb_addr() {
            aethercore::klog_info!(
                "Heap: DTB at {:#x} — dynamic memory regions should be added via hotplug",
                dtb_phys
            );
        }
    }
}

pub(crate) fn heap_ready() -> bool {
    HEAP_READY.load(Ordering::Acquire)
}

pub(super) fn finalize_heap_bootstrap() {
    if !PENDING_HEAP_FINALIZE.swap(false, Ordering::Relaxed) {
        return;
    }

    let virt_addr = PENDING_HEAP_VIRT_ADDR.load(Ordering::Relaxed);
    let phys_addr = PENDING_HEAP_PHYS_ADDR.load(Ordering::Relaxed);
    let actual_size = PENDING_HEAP_ACTUAL_SIZE.load(Ordering::Relaxed);
    let best_len = PENDING_HEAP_BEST_LEN.load(Ordering::Relaxed);
    aethercore::klog_info!(
        "Heap: virt={:#x} phys={:#x} size={} MiB ({} MiB available)",
        virt_addr,
        phys_addr,
        actual_size / BYTES_PER_MIB,
        best_len / BYTES_PER_MIB
    );

    let compaction_pages = PENDING_COMPACTION_PAGES.swap(0, Ordering::Relaxed);
    if compaction_pages != 0 {
        let compaction_base = PENDING_COMPACTION_BASE.swap(0, Ordering::Relaxed);
        aethercore::modules::allocators::advanced::register_compaction_candidate(
            compaction_base,
            compaction_pages,
        );
        aethercore::klog_info!(
            "Heap: {} MiB remainder registered for memory compaction",
            (compaction_pages * 4096) / BYTES_PER_MIB
        );
    }
}
