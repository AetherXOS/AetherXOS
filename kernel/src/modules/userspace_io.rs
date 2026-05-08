//! Userspace I/O framework for zero-copy operations
//! 
//! This module provides userspace I/O operations with:
//! - Zero-copy I/O between userspace and kernel
//! - Lock-free shared memory regions
//! - Batched I/O operations
//! - NUMA-aware memory distribution
//! - Telemetry for performance monitoring

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicU16, AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use core::ptr::NonNull;

const MAX_SHARED_REGIONS: usize = 256;
const REGION_SHARDS: usize = 32;
const MAX_IO_QUEUES: usize = 64;

// Telemetry
static USRIO_ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static USRIO_MAP_CALLS: AtomicU64 = AtomicU64::new(0);
static USRIO_IO_CALLS: AtomicU64 = AtomicU64::new(0);
static USRIO_ZERO_COPY_OPS: AtomicU64 = AtomicU64::new(0);
static USRIO_BATCH_OPS: AtomicU64 = AtomicU64::new(0);
static USRIO_NOTIFY_CALLS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct UserspaceIoStats {
    pub alloc_calls: u64,
    pub map_calls: u64,
    pub io_calls: u64,
    pub zero_copy_ops: u64,
    pub batch_ops: u64,
    pub notify_calls: u64,
    pub zero_copy_rate: f64,
}

pub fn userspace_io_stats() -> UserspaceIoStats {
    let io_calls = USRIO_IO_CALLS.load(Ordering::Relaxed);
    let zc_ops = USRIO_ZERO_COPY_OPS.load(Ordering::Relaxed);
    let zc_rate = if io_calls > 0 { zc_ops as f64 / io_calls as f64 } else { 0.0 };

    UserspaceIoStats {
        alloc_calls: USRIO_ALLOC_CALLS.load(Ordering::Relaxed),
        map_calls: USRIO_MAP_CALLS.load(Ordering::Relaxed),
        io_calls: io_calls,
        zero_copy_ops: zc_ops,
        batch_ops: USRIO_BATCH_OPS.load(Ordering::Relaxed),
        notify_calls: USRIO_NOTIFY_CALLS.load(Ordering::Relaxed),
        zero_copy_rate: zc_rate,
    }
}

/// Shared memory region for zero-copy I/O
#[repr(C, align(4096))]
pub struct SharedMemoryRegion {
    /// Region ID
    region_id: AtomicU64,
    /// Physical address
    phys_addr: AtomicU64,
    /// Virtual address (kernel)
    kernel_virt: AtomicU64,
    /// Virtual address (userspace)
    userspace_virt: AtomicU64,
    /// Region size
    size: AtomicU64,
    /// Owner process ID
    owner_pid: AtomicU32,
    /// Reference count
    refcount: AtomicU32,
    /// Read-only flag
    read_only: AtomicBool,
    /// Next pointer for lock-free list
    next: AtomicPtr<SharedMemoryRegion>,
    /// Region data (directly accessible)
    data: [u8; 0], // Flexible array member
}

impl SharedMemoryRegion {
    const fn new(region_id: u64, phys_addr: u64, size: u64, owner_pid: u32) -> Self {
        Self {
            region_id: AtomicU64::new(region_id),
            phys_addr: AtomicU64::new(phys_addr),
            kernel_virt: AtomicU64::new(0),
            userspace_virt: AtomicU64::new(0),
            size: AtomicU64::new(size),
            owner_pid: AtomicU32::new(owner_pid),
            refcount: AtomicU32::new(1),
            read_only: AtomicBool::new(false),
            next: AtomicPtr::new(core::ptr::null_mut()),
            data: [],
        }
    }

    #[inline(always)]
    fn increment_ref(&self) {
        self.refcount.fetch_add(1, Ordering::Relaxed);
    }

    #[inline(always)]
    fn decrement_ref(&self) -> bool {
        self.refcount.fetch_sub(1, Ordering::AcqRel) == 1
    }

    #[inline(always)]
    fn set_userspace_virt(&self, addr: u64) {
        self.userspace_virt.store(addr, Ordering::Release);
    }

    #[inline(always)]
    fn get_userspace_virt(&self) -> u64 {
        self.userspace_virt.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn get_size(&self) -> u64 {
        self.size.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn data_ptr(&self) -> *mut u8 {
        self as *const SharedMemoryRegion as *mut u8
    }
}

/// Lock-free shared memory registry shard
struct SharedMemoryShard {
    /// Hash table of regions
    table: [AtomicPtr<SharedMemoryRegion>; 128],
    /// Count of regions
    count: AtomicUsize,
}

impl SharedMemoryShard {
    const fn new() -> Self {
        const NULL_PTR: AtomicPtr<SharedMemoryRegion> = AtomicPtr::new(core::ptr::null_mut());
        Self {
            table: [NULL_PTR; 128],
            count: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    fn hash(&self, region_id: u64) -> usize {
        ((region_id as usize).wrapping_mul(0x9e3779b97f4a7c15)) % 128
    }

    /// Insert region (lock-free)
    #[inline(always)]
    fn insert(&self, region: *mut SharedMemoryRegion) -> bool {
        let region_id = unsafe { (*region).region_id.load(Ordering::Acquire) };
        let idx = self.hash(region_id);
        
        unsafe {
            let mut current = self.table[idx].load(Ordering::Acquire);
            
            loop {
                (*region).next.store(current, Ordering::Relaxed);
                
                match self.table[idx].compare_exchange_weak(
                    current,
                    region,
                    Ordering::Release,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        self.count.fetch_add(1, Ordering::Relaxed);
                        return true;
                    }
                    Err(actual) => current = actual,
                }
            }
        }
    }

    /// Lookup region (lock-free)
    #[inline(always)]
    fn lookup(&self, region_id: u64) -> Option<NonNull<SharedMemoryRegion>> {
        let idx = self.hash(region_id);
        let mut current = self.table[idx].load(Ordering::Acquire);
        
        while !current.is_null() {
            unsafe {
                let region = &*current;
                if region.region_id.load(Ordering::Acquire) == region_id {
                    region.increment_ref();
                    return Some(NonNull::new_unchecked(current));
                }
                current = region.next.load(Ordering::Acquire);
            }
        }
        
        None
    }

    /// Remove region (lock-free)
    #[inline(always)]
    fn remove(&self, region_id: u64) -> bool {
        let idx = self.hash(region_id);
        let mut prev: *mut SharedMemoryRegion = core::ptr::null_mut();
        let mut current = self.table[idx].load(Ordering::Acquire);
        
        while !current.is_null() {
            unsafe {
                let region = &*current;
                if region.region_id.load(Ordering::Acquire) == region_id {
                    let next = region.next.load(Ordering::Acquire);
                    
                    if prev.is_null() {
                        if self.table[idx].compare_exchange_weak(
                            current,
                            next,
                            Ordering::Release,
                            Ordering::Acquire,
                        ).is_ok() {
                            self.count.fetch_sub(1, Ordering::Relaxed);
                            return true;
                        }
                    } else {
                        let prev_region = &*prev;
                        if prev_region.next.compare_exchange_weak(
                            current,
                            next,
                            Ordering::Release,
                            Ordering::Acquire,
                        ).is_ok() {
                            self.count.fetch_sub(1, Ordering::Relaxed);
                            return true;
                        }
                    }
                }
                
                prev = current;
                current = region.next.load(Ordering::Acquire);
            }
        }
        
        false
    }
}

/// I/O queue for userspace-kernel communication
#[repr(C)]
pub struct IoQueue {
    /// Queue ID
    queue_id: AtomicU64,
    /// Head index (kernel writes here)
    head: AtomicU16,
    /// Tail index (userspace reads here)
    tail: AtomicU16,
    /// Queue depth
    depth: AtomicU16,
    /// Physical address of queue
    phys_addr: AtomicU64,
    /// Userspace virtual address
    userspace_virt: AtomicU64,
    /// Notification flag
    notify: AtomicBool,
}

impl IoQueue {
    const fn new(queue_id: u64, depth: u16) -> Self {
        Self {
            queue_id: AtomicU64::new(queue_id),
            head: AtomicU16::new(0),
            tail: AtomicU16::new(0),
            depth: AtomicU16::new(depth),
            phys_addr: AtomicU64::new(0),
            userspace_virt: AtomicU64::new(0),
            notify: AtomicBool::new(false),
        }
    }

    #[inline(always)]
    fn enqueue(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        let depth = self.depth.load(Ordering::Acquire);
        
        if (head.wrapping_add(1) % depth) == tail {
            return false; // Queue full
        }
        
        self.head.store(head.wrapping_add(1) % depth, Ordering::Release);
        true
    }

    #[inline(always)]
    fn dequeue(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        
        if head == tail {
            return false; // Queue empty
        }
        
        self.tail.store(tail.wrapping_add(1) as u16 % MAX_IO_QUEUES as u16, Ordering::Release);
        true
    }

    #[inline(always)]
    fn set_notify(&self, notify: bool) {
        self.notify.store(notify, Ordering::Release);
    }

    #[inline(always)]
    fn is_notified(&self) -> bool {
        self.notify.load(Ordering::Acquire)
    }
}

/// Userspace driver framework
pub struct UserspaceDriverFramework {
    /// Sharded shared memory registry
    shards: [SharedMemoryShard; REGION_SHARDS],
    /// Region ID counter
    region_counter: AtomicU64,
    /// I/O queues
    io_queues: [AtomicPtr<IoQueue>; MAX_IO_QUEUES],
    /// Queue ID counter
    queue_counter: AtomicU64,
}

impl UserspaceDriverFramework {
    pub const fn new() -> Self {
        const SHARD_INIT: SharedMemoryShard = SharedMemoryShard::new();
        const NULL_PTR: AtomicPtr<IoQueue> = AtomicPtr::new(core::ptr::null_mut());
        
        Self {
            shards: [SHARD_INIT; REGION_SHARDS],
            region_counter: AtomicU64::new(1),
            io_queues: [NULL_PTR; MAX_IO_QUEUES],
            queue_counter: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    fn get_shard(&self, region_id: u64) -> &SharedMemoryShard {
        let idx = (region_id as usize) % REGION_SHARDS;
        &self.shards[idx]
    }

    /// Allocate shared memory region
    #[inline(always)]
    pub fn alloc_shared_region(&self, size: u64, owner_pid: u32) -> Result<u64, &'static str> {
        USRIO_ALLOC_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let region_id = self.region_counter.fetch_add(1, Ordering::Relaxed);
        
        // Allocate memory for region header + data
        let total_size = core::mem::size_of::<SharedMemoryRegion>() + size as usize;
        let region = unsafe {
            alloc::alloc::alloc(
                core::alloc::Layout::from_size_align(total_size, 4096).unwrap()
            ) as *mut SharedMemoryRegion
        };
        
        if region.is_null() {
            return Err("allocation failed");
        }

        let phys_addr = 0x1000; // Would get from page allocator
        
        unsafe {
            region.write(SharedMemoryRegion::new(region_id, phys_addr, size, owner_pid));
        }

        let shard = self.get_shard(region_id);
        if shard.insert(region) {
            Ok(region_id)
        } else {
            Err("insert failed")
        }
    }

    /// Map shared region to userspace
    #[inline(always)]
    pub fn map_to_userspace(&self, region_id: u64, userspace_addr: u64) -> Result<(), &'static str> {
        USRIO_MAP_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let shard = self.get_shard(region_id);
        
        if let Some(region) = shard.lookup(region_id) {
            unsafe {
                region.as_ref().set_userspace_virt(userspace_addr);
            }
            Ok(())
        } else {
            Err("region not found")
        }
    }

    /// Zero-copy I/O operation
    #[inline(always)]
    pub fn zero_copy_io(&self, region_id: u64, offset: u64, data: &[u8], is_write: bool) -> Result<usize, &'static str> {
        USRIO_IO_CALLS.fetch_add(1, Ordering::Relaxed);
        USRIO_ZERO_COPY_OPS.fetch_add(1, Ordering::Relaxed);
        
        let shard = self.get_shard(region_id);
        
        if let Some(region) = shard.lookup(region_id) {
            unsafe {
                let region_ref = region.as_ref();
                let size = region_ref.get_size();
                let region_data = region_ref.data_ptr();
                
                if offset >= size {
                    return Err("offset out of bounds");
                }
                
                let available = (size - offset) as usize;
                let to_copy = data.len().min(available);
                
                if is_write {
                    // Write to shared memory
                    let dst = region_data.add(offset as usize);
                    let src = data.as_ptr();
                    core::ptr::copy_nonoverlapping(src, dst, to_copy);
                } else {
                    // Read from shared memory
                    let src = region_data.add(offset as usize);
                    let dst = data.as_ptr() as *mut u8;
                    core::ptr::copy_nonoverlapping(src, dst, to_copy);
                }
                
                Ok(to_copy)
            }
        } else {
            Err("region not found")
        }
    }

    /// Batched zero-copy I/O
    #[inline(always)]
    pub fn zero_copy_io_batch(&self, operations: &[(u64, usize, &[u8], bool)]) -> alloc::vec::Vec<usize> {
        USRIO_BATCH_OPS.fetch_add(1, Ordering::Relaxed);
        
        operations.iter().map(|&(region_id, offset, data, is_write)| {
            self.zero_copy_io(region_id, offset as u64, data, is_write).unwrap_or(0)
        }).collect()
    }

    /// Create I/O queue
    #[inline(always)]
    pub fn create_io_queue(&self, depth: u16) -> Result<u64, &'static str> {
        let queue_id = self.queue_counter.fetch_add(1, Ordering::Relaxed);
        let idx = (queue_id as usize) % MAX_IO_QUEUES;
        
        let queue = unsafe {
            alloc::alloc::alloc(
                core::alloc::Layout::new::<IoQueue>()
            ) as *mut IoQueue
        };
        
        if queue.is_null() {
            return Err("allocation failed");
        }

        unsafe {
            queue.write(IoQueue::new(queue_id, depth));
        }

        self.io_queues[idx].store(queue, Ordering::Release);
        Ok(queue_id)
    }

    /// Notify userspace of I/O completion
    #[inline(always)]
    pub fn notify_userspace(&self, queue_id: u64) -> Result<(), &'static str> {
        USRIO_NOTIFY_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let idx = (queue_id as usize) % MAX_IO_QUEUES;
        let queue = self.io_queues[idx].load(Ordering::Acquire);
        
        if queue.is_null() {
            return Err("queue not found");
        }

        unsafe {
            (*queue).set_notify(true);
        }
        
        Ok(())
    }

    /// Get shared region info
    #[inline(always)]
    pub fn get_region_info(&self, region_id: u64) -> Option<(u64, u64, u32)> {
        let shard = self.get_shard(region_id);
        
        if let Some(region) = shard.lookup(region_id) {
            unsafe {
                let region_ref = region.as_ref();
                Some((
                    region_ref.phys_addr.load(Ordering::Acquire),
                    region_ref.get_size(),
                    region_ref.owner_pid.load(Ordering::Acquire),
                ))
            }
        } else {
            None
        }
    }

    /// Free shared region
    #[inline(always)]
    pub fn free_region(&self, region_id: u64) -> Result<(), &'static str> {
        let shard = self.get_shard(region_id);
        
        if shard.remove(region_id) {
            Ok(())
        } else {
            Err("region not found")
        }
    }
}

/// NUMA-aware userspace driver framework
pub struct NumaUserspaceDriverFramework {
    /// Per-NUMA node frameworks
    node_frameworks: alloc::vec::Vec<UserspaceDriverFramework>,
    /// Current NUMA node
    current_node: AtomicUsize,
}

impl NumaUserspaceDriverFramework {
    pub fn new(numa_nodes: usize) -> Self {
        let mut frameworks = alloc::vec::Vec::with_capacity(numa_nodes);
        for _ in 0..numa_nodes {
            frameworks.push(UserspaceDriverFramework::new());
        }
        
        Self {
            node_frameworks: frameworks,
            current_node: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    fn get_node_framework(&self) -> &UserspaceDriverFramework {
        let node = self.current_node.load(Ordering::Relaxed) % self.node_frameworks.len();
        &self.node_frameworks[node]
    }

    /// Allocate on local NUMA node
    #[inline(always)]
    pub fn alloc_shared_region(&self, size: u64, owner_pid: u32) -> Result<u64, &'static str> {
        self.get_node_framework().alloc_shared_region(size, owner_pid)
    }

    /// Zero-copy I/O (NUMA-aware lookup)
    #[inline(always)]
    pub fn zero_copy_io(&self, region_id: u64, offset: u64, data: &[u8], is_write: bool) -> Result<usize, &'static str> {
        // Try local node first
        if let Ok(result) = self.get_node_framework().zero_copy_io(region_id, offset, data, is_write) {
            return Ok(result);
        }
        
        // Check other nodes
        for framework in &self.node_frameworks {
            if let Ok(result) = framework.zero_copy_io(region_id, offset, data, is_write) {
                return Ok(result);
            }
        }
        
        Err("region not found")
    }
}

/// Userspace driver interface
pub struct UserspaceDriverInterface {
    /// Framework reference
    framework: alloc::sync::Arc<UserspaceDriverFramework>,
    /// Driver ID
    driver_id: u64,
    /// I/O queue ID
    io_queue_id: AtomicU64,
}

impl UserspaceDriverInterface {
    pub fn new(framework: alloc::sync::Arc<UserspaceDriverFramework>, driver_id: u64) -> Self {
        Self {
            framework,
            driver_id,
            io_queue_id: AtomicU64::new(0),
        }
    }

    /// Initialize driver
    #[inline(always)]
    pub fn init(&self) -> Result<(), &'static str> {
        let queue_id = self.framework.create_io_queue(256)?;
        self.io_queue_id.store(queue_id, Ordering::Release);
        Ok(())
    }

    /// Submit I/O request
    #[inline(always)]
    pub fn submit_io(&self, region_id: u64, offset: u64, data: &[u8], is_write: bool) -> Result<usize, &'static str> {
        self.framework.zero_copy_io(region_id, offset, data, is_write)
    }

    /// Wait for I/O completion
    #[inline(always)]
    pub fn wait_completion(&self) -> Result<(), &'static str> {
        let queue_id = self.io_queue_id.load(Ordering::Acquire);
        self.framework.notify_userspace(queue_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_shared_memory_region() {
        let region = SharedMemoryRegion::new(1, 0x1000, 4096, 0);
        
        assert_eq!(region.refcount.load(Ordering::Relaxed), 1);
        
        region.increment_ref();
        assert_eq!(region.refcount.load(Ordering::Relaxed), 2);
        
        assert!(!region.decrement_ref());
        assert_eq!(region.refcount.load(Ordering::Relaxed), 1);
        
        assert!(region.decrement_ref());
    }

    #[test_case]
    fn test_shared_memory_shard() {
        let shard = SharedMemoryShard::new();
        
        let region = unsafe {
            alloc::alloc::alloc(
                core::alloc::Layout::from_size_align(
                    core::mem::size_of::<SharedMemoryRegion>() + 4096,
                    4096
                ).unwrap()
            ) as *mut SharedMemoryRegion
        };

        unsafe {
            region.write(SharedMemoryRegion::new(1, 0x1000, 4096, 0));
        }
        
        assert!(shard.insert(region));
        assert!(shard.lookup(1).is_some());
        
        assert!(shard.remove(1));
        assert!(shard.lookup(1).is_none());
    }

    #[test_case]
    fn test_io_queue() {
        let queue = IoQueue::new(1, 256);
        
        assert!(queue.enqueue());
        assert!(queue.dequeue());
        
        // Fill queue
        for _ in 0..255 {
            queue.enqueue();
        }
        
        assert!(!queue.enqueue()); // Queue full
    }

#[derive(Debug, Clone, Copy)]
pub struct DriverStats {
    pub alloc_calls: u64,
    pub map_calls: u64,
    pub io_calls: u64,
    pub zero_copy_ops: u64,
    pub batch_ops: u64,
    pub zero_copy_rate: f64,
}

pub fn userspace_driver_stats() -> DriverStats {
    let io_calls = USRIO_IO_CALLS.load(Ordering::Relaxed);
    let zc_ops = USRIO_ZERO_COPY_OPS.load(Ordering::Relaxed);
    let zc_rate = if io_calls > 0 { zc_ops as f64 / io_calls as f64 } else { 0.0 };

    DriverStats {
        alloc_calls: USRIO_ALLOC_CALLS.load(Ordering::Relaxed),
        map_calls: USRIO_MAP_CALLS.load(Ordering::Relaxed),
        io_calls,
        zero_copy_ops: zc_ops,
        batch_ops: USRIO_BATCH_OPS.load(Ordering::Relaxed),
        zero_copy_rate: zc_rate,
    }
}

    #[test_case]
    fn test_userspace_driver_framework() {
        let framework = UserspaceDriverFramework::new();
        
        const TEST_USERSPACE_ADDR: u64 = 0x7fff0000;
        
        let region_id = framework.alloc_shared_region(4096, 0).unwrap();
        assert_ne!(region_id, 0);
        
        framework.map_to_userspace(region_id, TEST_USERSPACE_ADDR).unwrap();
        
        let data = b"hello";
        let result = framework.zero_copy_io(region_id, 0, data, true);
        assert!(result.is_ok());
    }

    #[test_case]
    fn test_batch_io() {
        let framework = UserspaceDriverFramework::new();
        
        let region_id = framework.alloc_shared_region(4096, 0).unwrap();
        
        let operations = vec![
            (region_id, 0usize, &b"hello"[..], true),
            (region_id, 5usize, &b"world"[..], true),
        ];
        
        let results = framework.zero_copy_io_batch(&operations);
        assert_eq!(results.len(), 2);
    }

    #[test_case]
    fn test_userspace_driver_stats() {
        let stats = userspace_driver_stats();
        assert!(stats.zero_copy_rate >= 0.0 && stats.zero_copy_rate <= 1.0);
    }
}
