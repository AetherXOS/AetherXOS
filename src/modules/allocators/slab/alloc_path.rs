use super::*;

impl SlabAllocator {
    pub(super) unsafe fn try_refill_cache(
        &self,
        cpu_id: usize,
        idx: usize,
        block_size: usize,
    ) -> *mut u8 {
        let Some(refill_layout) = refill_layout_for_block_size(block_size) else {
            SLAB_REFILL_FAILURES.fetch_add(1, Ordering::Relaxed);
            return core::ptr::null_mut();
        };

        // Safety: caller holds allocator invariants and `refill_layout` comes from a validated size class.
        let memory = unsafe { self.fallback_allocator.alloc(refill_layout) };
        if memory.is_null() {
            SLAB_REFILL_FAILURES.fetch_add(1, Ordering::Relaxed);
            return core::ptr::null_mut();
        }

        let block_count = refill_layout.size() / block_size;
        if block_count == 0 {
            // Safety: `memory` was allocated from the same fallback allocator above.
            unsafe { self.fallback_allocator.dealloc(memory, refill_layout) };
            SLAB_REFILL_FAILURES.fetch_add(1, Ordering::Relaxed);
            return core::ptr::null_mut();
        }

        if !self.register_segment(
            idx,
            memory as usize,
            refill_layout.size(),
            refill_layout.align(),
            block_count,
        ) {
            // Safety: `memory` was allocated from the same fallback allocator above.
            unsafe { self.fallback_allocator.dealloc(memory, refill_layout) };
            SLAB_SEGMENT_TRACK_FAILURES.fetch_add(1, Ordering::Relaxed);
            SLAB_REFILL_FAILURES.fetch_add(1, Ordering::Relaxed);
            return core::ptr::null_mut();
        }

        let ptr = {
            let mut cache = self.caches[cpu_id][idx].lock();
            let mut current = memory;
            for _ in 0..block_count {
                // Safety: `current` walks the freshly allocated refill segment in block-sized steps.
                unsafe { cache.dealloc(current) };
                // Safety: `current` remains within the refill segment bounds for `block_count` iterations.
                current = unsafe { current.add(block_size) };
            }
            // Safety: cache invariants are maintained by the surrounding allocator lock.
            unsafe { cache.alloc() }
        };

        if !ptr.is_null() {
            let _ = self.note_segment_alloc(idx, ptr);
        }
        ptr
    }

    pub(super) unsafe fn steal_from_other_cpus(&self, cpu_id: usize, idx: usize) -> *mut u8 {
        let mut candidate = cpu_id + 1;
        for _ in 0..MAX_CPUS {
            let other = candidate % MAX_CPUS;
            candidate = candidate.wrapping_add(1);

            if other == cpu_id {
                continue;
            }

            let mut donor = self.caches[other][idx].lock();
            // Safety: donor cache is locked and maintains its internal free-list invariants.
            let ptr = unsafe { donor.steal_one() };
            if !ptr.is_null() {
                return ptr;
            }
        }

        core::ptr::null_mut()
    }

    pub(super) unsafe fn alloc_small_class(&self, idx: usize, layout: Layout) -> *mut u8 {
        let cpu_id = Self::effective_cpu_id();
        let block_size = BLOCK_SIZES[idx];

        let fast_ptr = {
            let mut cache = self.caches[cpu_id][idx].lock();
            // Safety: cache invariants are maintained by the surrounding allocator lock.
            unsafe { cache.alloc() }
        };
        if !fast_ptr.is_null() {
            let _ = self.note_segment_alloc(idx, fast_ptr);
            SLAB_ALLOC_FAST_HITS.fetch_add(1, Ordering::Relaxed);
            return fast_ptr;
        }

        let refill_ptr = unsafe { self.try_refill_cache(cpu_id, idx, block_size) };
        if !refill_ptr.is_null() {
            SLAB_ALLOC_REFILL_HITS.fetch_add(1, Ordering::Relaxed);
            return refill_ptr;
        }

        if SLAB_ENABLE_CROSS_CPU_STEAL.load(Ordering::Relaxed) {
            let stolen_ptr = unsafe { self.steal_from_other_cpus(cpu_id, idx) };
            if !stolen_ptr.is_null() {
                let _ = self.note_segment_alloc(idx, stolen_ptr);
                SLAB_ALLOC_STEAL_HITS.fetch_add(1, Ordering::Relaxed);
                return stolen_ptr;
            }
        }

        SLAB_ALLOC_FALLBACK_HITS.fetch_add(1, Ordering::Relaxed);
        // Safety: caller holds allocator invariants and `layout` is the original request.
        unsafe { self.fallback_allocator.alloc(layout) }
    }

    pub(super) unsafe fn dealloc_small_class(&self, ptr: *mut u8, layout: Layout, idx: usize) {
        let block_size = BLOCK_SIZES[idx];
        if !is_block_aligned(ptr, block_size) {
            // Safety: untracked pointers are returned to the fallback allocator that owns them.
            unsafe { self.fallback_allocator.dealloc(ptr, layout) };
            return;
        }

        let (tracked, reclaim_slot) = self.note_segment_free_and_maybe_reclaim(idx, ptr);
        if !tracked {
            // Safety: untracked pointers are returned to the fallback allocator that owns them.
            unsafe { self.fallback_allocator.dealloc(ptr, layout) };
            return;
        }

        let cpu_id = Self::effective_cpu_id();
        let cache_limit = SLAB_CACHE_LIMIT.load(Ordering::Relaxed);
        let mut pressure_reclaim = false;
        {
            let mut cache = self.caches[cpu_id][idx].lock();
            // Safety: tracked pointers match this size class and cache invariants are lock-protected.
            unsafe { cache.dealloc(ptr) };
            if cache.count > cache_limit {
                pressure_reclaim = true;
            }
        }

        if let Some(slot) = reclaim_slot {
            let _ = self.sample_reclaim_queue_depth(idx);
            self.try_reclaim_segment(idx, slot);
            return;
        }

        self.maybe_run_pressure_reclaim(idx, pressure_reclaim);
    }
}
