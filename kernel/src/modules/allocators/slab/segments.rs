use super::*;

pub(super) struct FreeBlock {
    pub(super) next: Option<NonNull<FreeBlock>>,
}

pub(super) struct SCache {
    pub(super) free_list: Option<NonNull<FreeBlock>>,
    pub(super) count: usize,
}

impl SCache {
    pub(super) const fn new() -> Self {
        Self {
            free_list: None,
            count: 0,
        }
    }

    pub(super) unsafe fn alloc(&mut self) -> *mut u8 {
        if let Some(node) = self.free_list {
            // Safety: `free_list` nodes are created by `dealloc` and remain valid while cached.
            self.free_list = unsafe { node.as_ref().next };
            self.count -= 1;
            node.as_ptr() as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    pub(super) unsafe fn dealloc(&mut self, ptr: *mut u8) {
        // Safety: callers only return slab-managed block pointers for this cache.
        let mut new_node = unsafe { NonNull::new_unchecked(ptr as *mut FreeBlock) };
        // Safety: `new_node` points at the block being returned to the freelist.
        unsafe { new_node.as_mut().next = self.free_list };
        self.free_list = Some(new_node);
        self.count += 1;
    }

    pub(super) unsafe fn steal_one(&mut self) -> *mut u8 {
        // Safety: this just forwards to the cache's own freelist pop path.
        unsafe { self.alloc() }
    }

    pub(super) unsafe fn drain_range(&mut self, start: usize, end: usize) -> usize {
        let mut removed = 0usize;
        let mut prev: Option<NonNull<FreeBlock>> = None;
        let mut current = self.free_list;

        while let Some(node) = current {
            // Safety: nodes reachable from `free_list` are valid until removed from the list.
            let next = unsafe { node.as_ref().next };
            let addr = node.as_ptr() as usize;
            let in_range = addr >= start && addr < end;

            if in_range {
                if let Some(mut prev_node) = prev {
                    // Safety: `prev_node` remains linked in the same freelist we are mutating.
                    unsafe { prev_node.as_mut().next = next };
                } else {
                    self.free_list = next;
                }
                self.count = self.count.saturating_sub(1);
                removed = removed.saturating_add(1);
            } else {
                prev = Some(node);
            }

            current = next;
        }

        removed
    }
}

#[derive(Clone, Copy)]
pub(super) struct SlabSegment {
    pub(super) active: bool,
    pub(super) reclaiming: bool,
    pub(super) class_idx: usize,
    pub(super) base: usize,
    pub(super) alloc_size: usize,
    pub(super) alloc_align: usize,
    pub(super) total_blocks: usize,
    pub(super) free_blocks: usize,
}

impl SlabSegment {
    pub(super) const EMPTY: Self = Self {
        active: false,
        reclaiming: false,
        class_idx: 0,
        base: 0,
        alloc_size: 0,
        alloc_align: 0,
        total_blocks: 0,
        free_blocks: 0,
    };

    #[inline(always)]
    pub(super) fn end(&self) -> usize {
        self.base.saturating_add(self.alloc_size)
    }

    #[inline(always)]
    pub(super) fn contains(&self, ptr: usize) -> bool {
        ptr >= self.base && ptr < self.end()
    }
}
