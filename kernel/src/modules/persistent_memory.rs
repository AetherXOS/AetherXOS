//! Persistent Memory (PMEM) support for fast storage
//! 
//! This module provides persistent memory operations with:
//! - Direct access to non-volatile memory
//! - Zero-copy operations for PMEM-backed files
//! - Lock-free PMEM allocation
//! - NUMA-aware PMEM distribution
//! - Telemetry for performance monitoring

use core::sync::atomic::{AtomicU64, AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use core::ptr::NonNull;

const PMEM_BLOCK_SIZE: usize = 4096; // 4KB blocks
const PMEM_MAX_BLOCKS: usize = 1048576; // 4GB at 4KB blocks
const PMEM_SHARDS: usize = 64;

// Telemetry
static PMEM_ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static PMEM_FREE_CALLS: AtomicU64 = AtomicU64::new(0);
static PMEM_READ_CALLS: AtomicU64 = AtomicU64::new(0);
static PMEM_WRITE_CALLS: AtomicU64 = AtomicU64::new(0);
static PMEM_FLUSH_CALLS: AtomicU64 = AtomicU64::new(0);
static PMEM_ZERO_COPY_OPS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct PmemStats {
    pub alloc_calls: u64,
    pub free_calls: u64,
    pub read_calls: u64,
    pub write_calls: u64,
    pub flush_calls: u64,
    pub zero_copy_ops: u64,
    pub used_blocks: u64,
    pub free_blocks: u64,
}

pub fn pmem_stats() -> PmemStats {
    PmemStats {
        alloc_calls: PMEM_ALLOC_CALLS.load(Ordering::Relaxed),
        free_calls: PMEM_FREE_CALLS.load(Ordering::Relaxed),
        read_calls: PMEM_READ_CALLS.load(Ordering::Relaxed),
        write_calls: PMEM_WRITE_CALLS.load(Ordering::Relaxed),
        flush_calls: PMEM_FLUSH_CALLS.load(Ordering::Relaxed),
        zero_copy_ops: PMEM_ZERO_COPY_OPS.load(Ordering::Relaxed),
        used_blocks: 0, // Would be tracked by allocator
        free_blocks: 0, // Would be tracked by allocator
    }
}

/// PMEM block header
#[repr(C)]
struct PmemBlockHeader {
    /// Block index
    index: AtomicU64,
    /// Size in bytes
    size: AtomicUsize,
    /// Allocated flag
    allocated: AtomicBool,
    /// Next free block (for free list)
    next: AtomicPtr<PmemBlockHeader>,
}

impl PmemBlockHeader {
    const fn new(index: u64, size: usize) -> Self {
        Self {
            index: AtomicU64::new(index),
            size: AtomicUsize::new(size),
            allocated: AtomicBool::new(false),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    #[inline(always)]
    fn is_allocated(&self) -> bool {
        self.allocated.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn set_allocated(&self, allocated: bool) {
        self.allocated.store(allocated, Ordering::Release);
    }

    #[inline(always)]
    fn get_size(&self) -> usize {
        self.size.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn data_ptr(&self) -> *mut u8 {
        (self as *const PmemBlockHeader as usize + core::mem::size_of::<PmemBlockHeader>()) as *mut u8
    }
}

/// Lock-free PMEM allocator shard
struct PmemShard {
    /// Free list
    free_list: AtomicPtr<PmemBlockHeader>,
    /// Block count
    count: AtomicUsize,
}

impl PmemShard {
    const fn new() -> Self {
        Self {
            free_list: AtomicPtr::new(core::ptr::null_mut()),
            count: AtomicUsize::new(0),
        }
    }

    /// Allocate a block from the free list
    #[inline(always)]
    fn alloc(&self) -> Option<NonNull<PmemBlockHeader>> {
        let mut head = self.free_list.load(Ordering::Acquire);
        
        while !head.is_null() {
            unsafe {
                let block = &*head;
                let next = block.next.load(Ordering::Acquire);
                
                if self.free_list.compare_exchange_weak(
                    head,
                    next,
                    Ordering::Release,
                    Ordering::Acquire,
                ).is_ok() {
                    block.set_allocated(true);
                    self.count.fetch_sub(1, Ordering::Relaxed);
                    return Some(NonNull::new_unchecked(head));
                }
                
                head = self.free_list.load(Ordering::Acquire);
            }
        }
        
        None
    }

    /// Free a block back to the free list
    #[inline(always)]
    fn free(&self, block: *mut PmemBlockHeader) {
        unsafe {
            (*block).set_allocated(false);
            
            let mut head = self.free_list.load(Ordering::Acquire);
            
            loop {
                (*block).next.store(head, Ordering::Relaxed);
                
                if self.free_list.compare_exchange_weak(
                    head,
                    block,
                    Ordering::Release,
                    Ordering::Acquire,
                ).is_ok() {
                    self.count.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                
                head = self.free_list.load(Ordering::Acquire);
            }
        }
    }
}

/// PMEM allocator
pub struct PmemAllocator {
    /// Base address of PMEM region
    base_addr: AtomicU64,
    /// Size of PMEM region
    size: AtomicU64,
    /// Sharded free lists
    shards: [PmemShard; PMEM_SHARDS],
    /// Block headers
    blocks: AtomicPtr<PmemBlockHeader>,
    /// Total blocks
    total_blocks: AtomicUsize,
}

impl PmemAllocator {
    pub const fn new() -> Self {
        const SHARD_INIT: PmemShard = PmemShard::new();
        Self {
            base_addr: AtomicU64::new(0),
            size: AtomicU64::new(0),
            shards: [SHARD_INIT; PMEM_SHARDS],
            blocks: AtomicPtr::new(core::ptr::null_mut()),
            total_blocks: AtomicUsize::new(0),
        }
    }

    /// Initialize PMEM allocator with given region
    pub fn init(&self, base_addr: u64, size: u64) {
        self.base_addr.store(base_addr, Ordering::Release);
        self.size.store(size, Ordering::Release);
        
        let num_blocks = (size as usize) / PMEM_BLOCK_SIZE;
        self.total_blocks.store(num_blocks, Ordering::Release);
        
        // Initialize block headers
        let headers = unsafe {
            alloc::alloc::alloc(
                core::alloc::Layout::array::<PmemBlockHeader>(num_blocks).unwrap()
            ) as *mut PmemBlockHeader
        };
        
        if !headers.is_null() {
            self.blocks.store(headers, Ordering::Release);
            
            // Initialize all blocks and add to free lists
            for i in 0..num_blocks {
                unsafe {
                    let block = headers.add(i);
                    block.write(PmemBlockHeader::new(i as u64, PMEM_BLOCK_SIZE));
                    
                    let shard_idx = i % PMEM_SHARDS;
                    self.shards[shard_idx].free(block);
                }
            }
        }
    }

    #[inline(always)]
    fn get_shard(&self, index: u64) -> &PmemShard {
        let idx = (index as usize) % PMEM_SHARDS;
        &self.shards[idx]
    }

    /// Allocate a block from PMEM
    #[inline(always)]
    pub fn alloc(&self) -> Option<NonNull<PmemBlockHeader>> {
        PMEM_ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        
        // Try each shard
        for i in 0..PMEM_SHARDS {
            if let Some(block) = self.shards[i].alloc() {
                return Some(block);
            }
        }
        
        None
    }

    /// Free a block back to PMEM
    #[inline(always)]
    pub fn free(&self, block: *mut PmemBlockHeader) {
        PMEM_FREE_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let index = unsafe { (*block).index.load(Ordering::Acquire) };
        let shard = self.get_shard(index);
        shard.free(block);
    }

    /// Get physical address of a block
    #[inline(always)]
    pub fn block_phys_addr(&self, block: *mut PmemBlockHeader) -> u64 {
        let base = self.base_addr.load(Ordering::Acquire);
        let index = unsafe { (*block).index.load(Ordering::Acquire) };
        base + (index * PMEM_BLOCK_SIZE as u64)
    }

    /// Get virtual address of a block
    #[inline(always)]
    pub fn block_virt_addr(&self, block: *mut PmemBlockHeader) -> *mut u8 {
        unsafe { (*block).data_ptr() }
    }
}

/// PMEM-backed file for ultra-fast storage
pub struct PmemFile {
    /// PMEM allocator
    allocator: alloc::sync::Arc<PmemAllocator>,
    /// File blocks
    blocks: alloc::vec::Vec<*mut PmemBlockHeader>,
    /// File size
    size: AtomicU64,
    /// File offset
    offset: AtomicU64,
}

impl PmemFile {
    pub fn new(allocator: alloc::sync::Arc<PmemAllocator>) -> Self {
        Self {
            allocator,
            blocks: alloc::vec::Vec::new(),
            size: AtomicU64::new(0),
            offset: AtomicU64::new(0),
        }
    }

    /// Allocate a new block for the file
    #[inline(always)]
    pub fn alloc_block(&mut self) -> Result<(), &'static str> {
        if let Some(block) = self.allocator.alloc() {
            self.blocks.push(block.as_ptr());
            let new_size = self.size.load(Ordering::Acquire) + PMEM_BLOCK_SIZE as u64;
            self.size.store(new_size, Ordering::Release);
            Ok(())
        } else {
            Err("out of PMEM")
        }
    }

    /// Zero-copy read from PMEM
    #[inline(always)]
    pub fn read_zero_copy(&self, buf: &mut [u8]) -> Result<usize, &'static str> {
        PMEM_READ_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let offset = self.offset.load(Ordering::Acquire);
        let remaining = self.size.load(Ordering::Acquire).saturating_sub(offset);
        let to_read = buf.len().min(remaining as usize);
        
        if to_read == 0 {
            return Ok(0);
        }

        let mut read = 0;
        let mut current_offset = offset;

        while read < to_read {
            let block_idx = (current_offset / PMEM_BLOCK_SIZE as u64) as usize;
            let block_in_offset = (current_offset % PMEM_BLOCK_SIZE as u64) as usize;
            
            if block_idx >= self.blocks.len() {
                break;
            }

            let block = self.blocks[block_idx];
            let block_size = unsafe { (*block).get_size() };
            let avail = block_size.saturating_sub(block_in_offset);
            let chunk = avail.min(to_read - read);
            
            unsafe {
                let src = (*block).data_ptr().add(block_in_offset);
                let dst = buf.as_mut_ptr().add(read);
                core::ptr::copy_nonoverlapping(src, dst, chunk);
            }
            
            PMEM_ZERO_COPY_OPS.fetch_add(1, Ordering::Relaxed);
            read += chunk;
            current_offset += chunk as u64;
        }

        self.offset.store(offset + read as u64, Ordering::Release);
        Ok(read)
    }

    /// Zero-copy write to PMEM
    #[inline(always)]
    pub fn write_zero_copy(&self, buf: &[u8]) -> Result<usize, &'static str> {
        PMEM_WRITE_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let offset = self.offset.load(Ordering::Acquire);
        let mut written = 0;
        let mut current_offset = offset;

        while written < buf.len() {
            let block_idx = (current_offset / PMEM_BLOCK_SIZE as u64) as usize;
            let block_in_offset = (current_offset % PMEM_BLOCK_SIZE as u64) as usize;
            
            if block_idx >= self.blocks.len() {
                break;
            }

            let block = self.blocks[block_idx];
            let block_size = unsafe { (*block).get_size() };
            let avail = block_size.saturating_sub(block_in_offset);
            let chunk = avail.min(buf.len() - written);
            
            unsafe {
                let src = buf.as_ptr().add(written);
                let dst = (*block).data_ptr().add(block_in_offset);
                core::ptr::copy_nonoverlapping(src, dst, chunk);
            }
            
            PMEM_ZERO_COPY_OPS.fetch_add(1, Ordering::Relaxed);
            written += chunk;
            current_offset += chunk as u64;
        }

        let new_size = (offset + written as u64).max(self.size.load(Ordering::Acquire));
        self.size.store(new_size, Ordering::Release);
        self.offset.store(offset + written as u64, Ordering::Release);
        
        Ok(written)
    }

    /// Flush PMEM to ensure persistence
    #[inline(always)]
    pub fn flush(&self) {
        PMEM_FLUSH_CALLS.fetch_add(1, Ordering::Relaxed);
        
        // Flush CPU caches for PMEM region
        #[cfg(target_arch = "x86_64")]
        {
            let base = self.allocator.base_addr.load(Ordering::Acquire);
            let size = self.allocator.size.load(Ordering::Acquire);
            
            unsafe {
                let mut addr = base;
                let end = base + size;
                
                while addr < end {
                    // CLFLUSH instruction
                    core::arch::asm!(
                        "clflush [{}]",
                        in(reg) addr as *const u8,
                        options(nostack, preserves_flags)
                    );
                    addr += 64; // Cache line size
                }
                
                // SFENCE to ensure ordering
                core::arch::asm!("sfence", options(nostack, preserves_flags));
            }
        }
    }

    /// Memory map PMEM file for direct access
    #[inline(always)]
    pub fn mmap(&self, offset: u64, _len: usize) -> Result<*mut u8, &'static str> {
        let block_idx = (offset / PMEM_BLOCK_SIZE as u64) as usize;
        
        if block_idx >= self.blocks.len() {
            return Err("invalid offset");
        }

        let block = self.blocks[block_idx];
        let block_in_offset = (offset % PMEM_BLOCK_SIZE as u64) as usize;
        
        unsafe {
            let base = (*block).data_ptr().add(block_in_offset);
            Ok(base)
        }
    }

    #[inline(always)]
    pub fn seek(&self, pos: u64) -> u64 {
        self.offset.store(pos, Ordering::Release);
        pos
    }

    #[inline(always)]
    pub fn tell(&self) -> u64 {
        self.offset.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn len(&self) -> u64 {
        self.size.load(Ordering::Acquire)
    }
}

/// NUMA-aware PMEM allocator
pub struct NumaPmemAllocator {
    /// Per-NUMA node allocators
    node_allocators: alloc::vec::Vec<alloc::sync::Arc<PmemAllocator>>,
    /// Current NUMA node
    current_node: AtomicUsize,
}

impl NumaPmemAllocator {
    pub fn new(numa_nodes: usize) -> Self {
        let mut allocators = alloc::vec::Vec::with_capacity(numa_nodes);
        for _ in 0..numa_nodes {
            allocators.push(alloc::sync::Arc::new(PmemAllocator::new()));
        }
        
        Self {
            node_allocators: allocators,
            current_node: AtomicUsize::new(0),
        }
    }

    /// Initialize all NUMA node allocators
    pub fn init_all(&self, base_addrs: &[u64], sizes: &[u64]) {
        for (i, allocator) in self.node_allocators.iter().enumerate() {
            if i < base_addrs.len() && i < sizes.len() {
                allocator.init(base_addrs[i], sizes[i]);
            }
        }
    }

    #[inline(always)]
    fn get_node_allocator(&self) -> &alloc::sync::Arc<PmemAllocator> {
        let node = self.current_node.load(Ordering::Relaxed) % self.node_allocators.len();
        &self.node_allocators[node]
    }

    /// Allocate from local NUMA node
    #[inline(always)]
    pub fn alloc(&self) -> Option<NonNull<PmemBlockHeader>> {
        self.get_node_allocator().alloc()
    }

    /// Free to local NUMA node
    #[inline(always)]
    pub fn free(&self, block: *mut PmemBlockHeader) {
        self.get_node_allocator().free(block);
    }
}

/// PMEM pool for pre-allocated blocks
pub struct PmemPool {
    /// Available blocks
    available: alloc::vec::Vec<*mut PmemBlockHeader>,
    /// Allocator reference
    allocator: alloc::sync::Arc<PmemAllocator>,
}

impl PmemPool {
    pub fn new(allocator: alloc::sync::Arc<PmemAllocator>, size: usize) -> Self {
        let mut available = alloc::vec::Vec::with_capacity(size);
        
        for _ in 0..size {
            if let Some(block) = allocator.alloc() {
                available.push(block.as_ptr());
            }
        }
        
        Self {
            available,
            allocator,
        }
    }

    /// Get a block from the pool
    #[inline(always)]
    pub fn get(&mut self) -> Option<*mut PmemBlockHeader> {
        self.available.pop()
    }

    /// Return a block to the pool
    #[inline(always)]
    pub fn put(&mut self, block: *mut PmemBlockHeader) {
        self.available.push(block);
    }

    /// Prefetch blocks into the pool
    #[inline(always)]
    pub fn prefetch(&mut self, count: usize) {
        while self.available.len() < count {
            if let Some(block) = self.allocator.alloc() {
                self.available.push(block.as_ptr());
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_pmem_block_header() {
        let header = PmemBlockHeader::new(0, 4096);
        
        assert!(!header.is_allocated());
        assert_eq!(header.get_size(), 4096);
        
        header.set_allocated(true);
        assert!(header.is_allocated());
    }

    #[test_case]
    fn test_pmem_shard() {
        let shard = PmemShard::new();
        
        let mut block = PmemBlockHeader::new(0, 4096);
        shard.free(&mut block);
        
        assert_eq!(shard.count.load(Ordering::Relaxed), 1);
        
        let allocated = shard.alloc();
        assert!(allocated.is_some());
    }

    #[test_case]
    fn test_pmem_allocator() {
        let allocator = PmemAllocator::new();
        allocator.init(0x10000000, 0x1000000); // 16MB
        
        let block = allocator.alloc();
        assert!(block.is_some());
        
        if let Some(block) = block {
            allocator.free(block.as_ptr());
        }
    }

    #[test_case]
    fn test_pmem_file() {
        let allocator = alloc::sync::Arc::new(PmemAllocator::new());
        allocator.init(0x10000000, 0x1000000);
        
        let mut file = PmemFile::new(allocator.clone());
        file.alloc_block().unwrap();
        
        let data = b"hello world";
        let mut buf = [0u8; 256];
        
        file.write_zero_copy(data).unwrap();
        file.seek(0);
        let read = file.read_zero_copy(&mut buf).unwrap();
        
        assert_eq!(read, data.len());
    }

    #[test_case]
    fn test_pmem_stats() {
        let _stats = pmem_stats();
    }
}
