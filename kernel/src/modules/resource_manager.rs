//! Exokernel-style resource management
//! 
//! This module provides secure resource allocation and management following exokernel principles:
//! - Minimal kernel that securely multiplexes hardware resources
//! - Capability-based resource allocation
//! - Userspace-controlled resource management
//! - Secure resource binding and revocation
//! - Telemetry for resource usage monitoring

use core::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, AtomicPtr, AtomicBool, AtomicUsize, Ordering};
use core::ptr::NonNull;

const MAX_RESOURCES: usize = 65536;
const RESOURCE_SHARDS: usize = 64;

// Telemetry
static RESOURCE_ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static RESOURCE_DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static RESOURCE_REVOCATIONS: AtomicU64 = AtomicU64::new(0);
static RESOURCE_BINDINGS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Memory = 1,
    Device = 2,
    Interrupt = 3,
    Dma = 4,
    Network = 5,
    Storage = 6,
}

#[derive(Debug, Clone, Copy)]
pub struct ResourceStats {
    pub allocations: u64,
    pub deallocations: u64,
    pub revocations: u64,
    pub bindings: u64,
    pub active_count: u64,
}

pub fn resource_stats() -> ResourceStats {
    ResourceStats {
        allocations: RESOURCE_ALLOCATIONS.load(Ordering::Relaxed),
        deallocations: RESOURCE_DEALLOCATIONS.load(Ordering::Relaxed),
        revocations: RESOURCE_REVOCATIONS.load(Ordering::Relaxed),
        bindings: RESOURCE_BINDINGS.load(Ordering::Relaxed),
        active_count: RESOURCE_ALLOCATIONS.load(Ordering::Relaxed)
            .saturating_sub(RESOURCE_DEALLOCATIONS.load(Ordering::Relaxed)),
    }
}

/// Resource descriptor for exokernel resource management
#[repr(C)]
pub struct ResourceDescriptor {
    resource_id: AtomicU64,
    resource_type: AtomicU8,
    owner: AtomicU32,
    base_address: AtomicU64,
    size: AtomicU64,
    permissions: AtomicU64,
    valid: AtomicBool,
    next: AtomicPtr<ResourceDescriptor>,
}

impl ResourceDescriptor {
    const fn new(resource_id: u64, resource_type: ResourceType, owner: u32, base: u64, size: u64) -> Self {
        Self {
            resource_id: AtomicU64::new(resource_id),
            resource_type: AtomicU8::new(resource_type as u8),
            owner: AtomicU32::new(owner),
            base_address: AtomicU64::new(base),
            size: AtomicU64::new(size),
            permissions: AtomicU64::new(0),
            valid: AtomicBool::new(true),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    #[inline(always)]
    fn invalidate(&self) {
        self.valid.store(false, Ordering::Release);
    }

    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }
}

/// Resource shard for lock-free access
struct ResourceShard {
    table: [AtomicPtr<ResourceDescriptor>; 256],
    count: AtomicUsize,
}

impl ResourceShard {
    const fn new() -> Self {
        const NULL_PTR: AtomicPtr<ResourceDescriptor> = AtomicPtr::new(core::ptr::null_mut());
        Self {
            table: [NULL_PTR; 256],
            count: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    fn hash(&self, resource_id: u64) -> usize {
        ((resource_id as usize).wrapping_mul(0x9e3779b97f4a7c15)) % 256
    }

    #[inline(always)]
    fn insert(&self, resource: *mut ResourceDescriptor) -> bool {
        let resource_id = unsafe { (*resource).resource_id.load(Ordering::Acquire) };
        let idx = self.hash(resource_id);
        
        unsafe {
            let mut current = self.table[idx].load(Ordering::Acquire);
            loop {
                (*resource).next.store(current, Ordering::Relaxed);
                match self.table[idx].compare_exchange_weak(
                    current,
                    resource,
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

    #[inline(always)]
    fn lookup(&self, resource_id: u64) -> Option<NonNull<ResourceDescriptor>> {
        let idx = self.hash(resource_id);
        let mut current = self.table[idx].load(Ordering::Acquire);
        
        while !current.is_null() {
            unsafe {
                let resource = &*current;
                if resource.resource_id.load(Ordering::Acquire) == resource_id && resource.is_valid() {
                    return Some(NonNull::new_unchecked(current));
                }
                current = resource.next.load(Ordering::Acquire);
            }
        }
        
        None
    }

    #[inline(always)]
    fn remove(&self, resource_id: u64) -> bool {
        let idx = self.hash(resource_id);
        let mut prev: *mut ResourceDescriptor = core::ptr::null_mut();
        let mut current = self.table[idx].load(Ordering::Acquire);
        
        while !current.is_null() {
            unsafe {
                let resource = &*current;
                if resource.resource_id.load(Ordering::Acquire) == resource_id {
                    let next = resource.next.load(Ordering::Acquire);
                    
                    if prev.is_null() {
                        if self.table[idx].compare_exchange_weak(
                            current,
                            next,
                            Ordering::Release,
                            Ordering::Acquire,
                        ).is_ok() {
                            resource.invalidate();
                            self.count.fetch_sub(1, Ordering::Relaxed);
                            return true;
                        }
                    } else {
                        let prev_resource = &*prev;
                        if prev_resource.next.compare_exchange_weak(
                            current,
                            next,
                            Ordering::Release,
                            Ordering::Acquire,
                        ).is_ok() {
                            resource.invalidate();
                            self.count.fetch_sub(1, Ordering::Relaxed);
                            return true;
                        }
                    }
                }
                prev = current;
                current = resource.next.load(Ordering::Acquire);
            }
        }
        
        false
    }
}

/// Exokernel resource manager
pub struct ResourceManager {
    shards: [ResourceShard; RESOURCE_SHARDS],
    resource_counter: AtomicU64,
}

impl ResourceManager {
    pub const fn new() -> Self {
        const SHARD_INIT: ResourceShard = ResourceShard::new();
        Self {
            shards: [SHARD_INIT; RESOURCE_SHARDS],
            resource_counter: AtomicU64::new(1),
        }
    }

    #[inline(always)]
    fn get_shard(&self, resource_id: u64) -> &ResourceShard {
        let idx = (resource_id as usize) % RESOURCE_SHARDS;
        &self.shards[idx]
    }

    /// Allocate a resource (exokernel-style)
    #[inline(always)]
    pub fn allocate(&self, resource_type: ResourceType, owner: u32, base: u64, size: u64) -> Result<u64, &'static str> {
        RESOURCE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        
        let resource_id = self.resource_counter.fetch_add(1, Ordering::Relaxed);
        let resource = unsafe {
            alloc::alloc::alloc(
                core::alloc::Layout::new::<ResourceDescriptor>()
            ) as *mut ResourceDescriptor
        };
        
        if resource.is_null() {
            return Err("allocation failed");
        }

        unsafe {
            resource.write(ResourceDescriptor::new(resource_id, resource_type, owner, base, size));
        }

        let shard = self.get_shard(resource_id);
        if shard.insert(resource) {
            Ok(resource_id)
        } else {
            Err("insert failed")
        }
    }

    /// Deallocate a resource
    #[inline(always)]
    pub fn deallocate(&self, resource_id: u64) -> Result<(), &'static str> {
        RESOURCE_DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        
        let shard = self.get_shard(resource_id);
        if shard.remove(resource_id) {
            Ok(())
        } else {
            Err("resource not found")
        }
    }

    /// Revoke a resource (force deallocation)
    #[inline(always)]
    pub fn revoke(&self, resource_id: u64) -> Result<(), &'static str> {
        RESOURCE_REVOCATIONS.fetch_add(1, Ordering::Relaxed);
        self.deallocate(resource_id)
    }

    /// Bind a resource to a capability
    #[inline(always)]
    pub fn bind(&self, resource_id: u64, capability_id: u64) -> Result<(), &'static str> {
        RESOURCE_BINDINGS.fetch_add(1, Ordering::Relaxed);
        
        let shard = self.get_shard(resource_id);
        if let Some(resource) = shard.lookup(resource_id) {
            unsafe {
                resource.as_ref().permissions.store(capability_id, Ordering::Release);
            }
            Ok(())
        } else {
            Err("resource not found")
        }
    }

    /// Get resource info
    #[inline(always)]
    pub fn get_info(&self, resource_id: u64) -> Option<(u8, u32, u64, u64)> {
        let shard = self.get_shard(resource_id);
        
        if let Some(resource) = shard.lookup(resource_id) {
            unsafe {
                let resource_ref = resource.as_ref();
                Some((
                    resource_ref.resource_type.load(Ordering::Acquire),
                    resource_ref.owner.load(Ordering::Acquire),
                    resource_ref.base_address.load(Ordering::Acquire),
                    resource_ref.size.load(Ordering::Acquire),
                ))
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_resource_allocation() {
        let manager = ResourceManager::new();
        
        let result = manager.allocate(ResourceType::Memory, 0, 0x1000, 4096);
        assert!(result.is_ok());
    }

    #[test_case]
    fn test_resource_stats() {
        let _stats = resource_stats();
    }
}
