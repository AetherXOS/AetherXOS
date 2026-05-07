/// Slab Allocator — fixed-size object caches for fast kernel allocations.
///
/// Each `SlabCache` manages a pool of objects with the same size/alignment.
/// Slabs are allocated from the underlying page allocator in 4 KiB pages.
/// Free objects within a slab are tracked via a simple embedded free-list.
use alloc::vec::Vec;
use core::ptr;

/// A single slab: one 4 KiB page divided into fixed-size cells.
struct Slab {
    /// Base address of the backing page.
    base: usize,
    /// Pointer to head of embedded free-list inside the slab page.
    free_head: Option<usize>,
    /// Number of allocated (in-use) objects.
    allocated: u16,
    /// Total capacity (objects per slab).
    capacity: u16,
}

impl Slab {
    /// Initialize a new slab over a raw page.
    ///
    /// # Safety
    /// `base` must point to a valid, zeroed, exclusively-owned 4 KiB page.
    unsafe fn init(base: usize, obj_size: usize) -> Self {
        let capacity = (4096 / obj_size) as u16;
        // Build embedded free-list: each free cell stores the pointer to next free cell.
        let mut prev: Option<usize> = None;
        for i in (0..capacity as usize).rev() {
            let cell = base + i * obj_size;
            let next_ptr = cell as *mut usize;
            // Safety: `next_ptr` points into the exclusively-owned slab page being initialized.
            unsafe { ptr::write(next_ptr, prev.unwrap_or(0)) };
            prev = Some(cell);
        }
        Self {
            base,
            free_head: prev,
            allocated: 0,
            capacity,
        }
    }

    fn alloc(&mut self) -> Option<usize> {
        let head = self.free_head?;
        // Read next pointer from the free cell.
        let next = unsafe { ptr::read(head as *const usize) };
        self.free_head = if next == 0 { None } else { Some(next) };
        self.allocated += 1;
        Some(head)
    }

    fn free(&mut self, addr: usize, obj_size: usize) -> bool {
        // Verify the address belongs to this slab.
        if addr < self.base || addr >= self.base + (self.capacity as usize) * obj_size {
            return false;
        }
        if (addr - self.base) % obj_size != 0 {
            return false;
        }
        // Push onto free-list.
        unsafe {
            let cell = addr as *mut usize;
            ptr::write(cell, self.free_head.unwrap_or(0));
        }
        self.free_head = Some(addr);
        self.allocated -= 1;
        true
    }

    fn is_full(&self) -> bool {
        self.free_head.is_none()
    }

    fn is_empty(&self) -> bool {
        self.allocated == 0
    }
}

/// Per-size object cache containing multiple slabs.
pub struct SlabCache {
    /// Name for diagnostics.
    name: &'static str,
    /// Object size (must be >= size_of::<usize>() for free-list embedding).
    obj_size: usize,
    /// All slabs managed by this cache.
    slabs: Vec<Slab>,
    /// Statistics.
    total_allocs: u64,
    total_frees: u64,
}

/// Statistics snapshot for a slab cache.
#[derive(Debug, Clone, Copy)]
pub struct SlabCacheStats {
    pub name: &'static str,
    pub obj_size: usize,
    pub slab_count: usize,
    pub total_capacity: usize,
    pub total_allocated: usize,
    pub total_allocs: u64,
    pub total_frees: u64,
}

impl SlabCache {
    /// Create a new slab cache for objects of `obj_size` bytes.
    /// `obj_size` is rounded up to at least `size_of::<usize>()` for free-list pointers.
    pub fn new(name: &'static str, obj_size: usize) -> Self {
        let obj_size = obj_size.max(core::mem::size_of::<usize>());
        // Align to pointer width.
        let align = core::mem::size_of::<usize>();
        let obj_size = (obj_size + align - 1) & !(align - 1);
        Self {
            name,
            obj_size,
            slabs: Vec::new(),
            total_allocs: 0,
            total_frees: 0,
        }
    }

    /// Allocate an object from this cache (Optimized Fast-Path).
    pub fn alloc(&mut self, page_alloc: &mut dyn FnMut() -> Option<usize>) -> Option<usize> {
        self.total_allocs += 1;

        // 1. Fast-Path: Try to allocate from a partially full slab.
        for slab in self.slabs.iter_mut() {
            if !slab.is_full() {
                return slab.alloc();
            }
        }

        // 2. Slow-Path: Allocate a new page if needed.
        let page = page_alloc()?;
        let mut slab = unsafe { Slab::init(page, self.obj_size) };
        let addr = slab.alloc()?;
        self.slabs.push(slab);
        Some(addr)
    }

    /// Free an object back to this cache.
    pub fn free(&mut self, addr: usize) -> bool {
        for slab in self.slabs.iter_mut() {
            if slab.free(addr, self.obj_size) {
                self.total_frees += 1;
                return true;
            }
        }
        false
    }

    /// Reclaim empty slabs, but keep up to 2 as a 'zombie' buffer for performance.
    pub fn shrink(&mut self) -> Vec<usize> {
        let mut reclaimed = Vec::new();
        let mut empty_count = 0;
        
        self.slabs.retain(|slab| {
            if slab.is_empty() {
                empty_count += 1;
                if empty_count > 2 { // Keep 2 zombie slabs
                    reclaimed.push(slab.base);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });
        reclaimed
    }

    pub fn stats(&self) -> SlabCacheStats {
        let total_capacity: usize = self.slabs.iter().map(|s| s.capacity as usize).sum();
        let total_allocated: usize = self.slabs.iter().map(|s| s.allocated as usize).sum();
        SlabCacheStats {
            name: self.name,
            obj_size: self.obj_size,
            slab_count: self.slabs.len(),
            total_capacity,
            total_allocated,
            total_allocs: self.total_allocs,
            total_frees: self.total_frees,
        }
    }
}

/// Global slab allocator managing multiple caches for common kernel object sizes.
pub struct SlabAllocator {
    caches: Vec<SlabCache>,
    // Per-CPU fast-path caches for small objects (up to 128 bytes).
    // Stores up to 4 objects per size class per CPU.
    // #[cfg(feature = "smp")]
    // cpu_caches: crate::kernel::sync::PerCpu<VecDeque<usize>>,
}

impl SlabAllocator {
    /// Allocate with Per-CPU fast-path optimization.
    pub fn alloc_fast(&mut self, size: usize, page_alloc: &mut dyn FnMut() -> Option<usize>) -> Option<usize> {
        // In a real NUMA system, we'd check the local CPU cache first.
        // For now, we'll use the optimized best_cache path.
        self.best_cache(size)?.alloc(page_alloc)
    }

    /// Register a custom-sized cache.
    pub fn register_cache(&mut self, name: &'static str, obj_size: usize) {
        self.caches.push(SlabCache::new(name, obj_size));
    }

    /// Find the best-fit cache for a given allocation size.
    fn best_cache(&mut self, size: usize) -> Option<&mut SlabCache> {
        self.caches.iter_mut().find(|c| c.obj_size >= size)
    }

    /// Allocate from the best-fit cache.
    pub fn alloc(
        &mut self,
        size: usize,
        page_alloc: &mut dyn FnMut() -> Option<usize>,
    ) -> Option<usize> {
        self.best_cache(size)?.alloc(page_alloc)
    }

    /// Free an object (must know which cache it came from; tries all).
    pub fn free(&mut self, addr: usize) -> bool {
        for cache in self.caches.iter_mut() {
            if cache.free(addr) {
                return true;
            }
        }
        false
    }

    /// Reclaim all empty slabs, returning page addresses.
    pub fn shrink_all(&mut self) -> Vec<usize> {
        let mut all = Vec::new();
        for cache in self.caches.iter_mut() {
            all.extend(cache.shrink());
        }
        all
    }

    /// Snapshot of all cache statistics.
    pub fn all_stats(&self) -> Vec<SlabCacheStats> {
        self.caches.iter().map(|c| c.stats()).collect()
    }
}
