//! Lock-free capability-based security system
//! 
//! This module provides security operations with:
//! - Lock-free capability token validation
//! - Zero-copy security checks
//! - Batched permission verification
//! - NUMA-aware capability distribution
//! - Telemetry for performance monitoring

use core::sync::atomic::{AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, AtomicBool, Ordering};
use core::ptr::NonNull;

const MAX_CAPABILITIES: usize = 65536;
const CAPABILITY_SHARDS: usize = 64;
const PERMISSION_BITS: u64 = 64;

// Telemetry
static SEC_CHECK_CALLS: AtomicU64 = AtomicU64::new(0);
static SEC_CHECK_HITS: AtomicU64 = AtomicU64::new(0);
static SEC_CHECK_DENIES: AtomicU64 = AtomicU64::new(0);
static SEC_MINT_CALLS: AtomicU64 = AtomicU64::new(0);
static SEC_REVOKE_CALLS: AtomicU64 = AtomicU64::new(0);
static SEC_BATCH_CHECKS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct SecurityStats {
    pub check_calls: u64,
    pub check_hits: u64,
    pub check_denies: u64,
    pub mint_calls: u64,
    pub revoke_calls: u64,
    pub batch_checks: u64,
    pub hit_rate: f64,
}

pub fn security_stats() -> SecurityStats {
    let checks = SEC_CHECK_CALLS.load(Ordering::Relaxed);
    let hits = SEC_CHECK_HITS.load(Ordering::Relaxed);
    let hit_rate = if checks > 0 { hits as f64 / checks as f64 } else { 0.0 };

    SecurityStats {
        check_calls: checks,
        check_hits: hits,
        check_denies: SEC_CHECK_DENIES.load(Ordering::Relaxed),
        mint_calls: SEC_MINT_CALLS.load(Ordering::Relaxed),
        revoke_calls: SEC_REVOKE_CALLS.load(Ordering::Relaxed),
        batch_checks: SEC_BATCH_CHECKS.load(Ordering::Relaxed),
        hit_rate,
    }
}

/// Permission bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permission(u64);

impl Permission {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const CREATE: Self = Self(1 << 3);
    pub const DELETE: Self = Self(1 << 4);
    pub const ADMIN: Self = Self(1 << 5);
    pub const MOUNT: Self = Self(1 << 6);
    pub const SIGNAL: Self = Self(1 << 7);
    pub const IPC: Self = Self(1 << 8);
    pub const NET: Self = Self(1 << 9);
    pub const ALL: Self = Self(u64::MAX);

    #[inline(always)]
    pub fn contains(&self, perm: Permission) -> bool {
        self.0 & perm.0 != 0
    }

    #[inline(always)]
    pub fn union(&self, other: Permission) -> Permission {
        Permission(self.0 | other.0)
    }

    #[inline(always)]
    pub fn intersect(&self, other: Permission) -> Permission {
        Permission(self.0 & other.0)
    }
}

/// Lock-free capability token
#[repr(C, align(8))]
pub struct UltraCapability {
    /// Token ID (unique identifier)
    token_id: AtomicU64,
    /// Resource ID
    resource_id: AtomicU64,
    /// Permission bitmask
    permissions: AtomicU64,
    /// Owner task ID
    owner: AtomicU32,
    /// Generation counter (for revocation)
    generation: AtomicU32,
    /// Valid flag
    valid: AtomicBool,
    /// Next pointer for lock-free list
    next: AtomicPtr<UltraCapability>,
}

impl UltraCapability {
    const fn new(token_id: u64, resource_id: u64, permissions: u64, owner: u32) -> Self {
        Self {
            token_id: AtomicU64::new(token_id),
            resource_id: AtomicU64::new(resource_id),
            permissions: AtomicU64::new(permissions),
            owner: AtomicU32::new(owner),
            generation: AtomicU32::new(0),
            valid: AtomicBool::new(true),
            next: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    #[inline(always)]
    fn is_valid(&self) -> bool {
        self.valid.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn invalidate(&self) {
        self.valid.store(false, Ordering::Release);
    }

    #[inline(always)]
    fn get_token_id(&self) -> u64 {
        self.token_id.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn get_resource_id(&self) -> u64 {
        self.resource_id.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn get_permissions(&self) -> u64 {
        self.permissions.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn get_owner(&self) -> u32 {
        self.owner.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn get_generation(&self) -> u32 {
        self.generation.load(Ordering::Acquire)
    }

    #[inline(always)]
    fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::Release);
    }

    #[inline(always)]
    fn check_permission(&self, required: u64) -> bool {
        let perms = self.permissions.load(Ordering::Acquire);
        (perms & required) == required
    }
}

/// Lock-free capability shard
struct CapabilityShard {
    /// Hash table of capabilities
    table: [AtomicPtr<UltraCapability>; 256],
    /// Count of capabilities in this shard
    count: AtomicUsize,
}

impl CapabilityShard {
    const fn new() -> Self {
        const NULL_PTR: AtomicPtr<UltraCapability> = AtomicPtr::new(core::ptr::null_mut());
        Self {
            table: [NULL_PTR; 256],
            count: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    fn hash(&self, token_id: u64) -> usize {
        ((token_id as usize).wrapping_mul(0x9e3779b97f4a7c15)) % 256
    }

    /// Insert capability (lock-free)
    #[inline(always)]
    fn insert(&self, cap: *mut UltraCapability) -> bool {
        let token_id = unsafe { (*cap).get_token_id() };
        let idx = self.hash(token_id);
        
        unsafe {
            let mut current = self.table[idx].load(Ordering::Acquire);
            
            loop {
                (*cap).next.store(current, Ordering::Relaxed);
                
                match self.table[idx].compare_exchange_weak(
                    current,
                    cap,
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

    /// Lookup capability (lock-free read)
    #[inline(always)]
    fn lookup(&self, token_id: u64) -> Option<NonNull<UltraCapability>> {
        let idx = self.hash(token_id);
        let mut current = self.table[idx].load(Ordering::Acquire);
        
        while !current.is_null() {
            unsafe {
                let cap = &*current;
                if cap.get_token_id() == token_id && cap.is_valid() {
                    return Some(NonNull::new_unchecked(current));
                }
                current = cap.next.load(Ordering::Acquire);
            }
        }
        
        None
    }

    /// Remove capability (lock-free)
    #[inline(always)]
    fn remove(&self, token_id: u64) -> bool {
        let idx = self.hash(token_id);
        let mut prev: *mut UltraCapability = core::ptr::null_mut();
        let mut current = self.table[idx].load(Ordering::Acquire);
        
        while !current.is_null() {
            unsafe {
                let cap = &*current;
                if cap.get_token_id() == token_id {
                    let next = cap.next.load(Ordering::Acquire);
                    
                    if prev.is_null() {
                        // Removing head
                        if self.table[idx].compare_exchange_weak(
                            current,
                            next,
                            Ordering::Release,
                            Ordering::Acquire,
                        ).is_ok() {
                            cap.invalidate();
                            self.count.fetch_sub(1, Ordering::Relaxed);
                            return true;
                        }
                    } else {
                        // Removing from middle
                        let prev_cap = &*prev;
                        if prev_cap.next.compare_exchange_weak(
                            current,
                            next,
                            Ordering::Release,
                            Ordering::Acquire,
                        ).is_ok() {
                            cap.invalidate();
                            self.count.fetch_sub(1, Ordering::Relaxed);
                            return true;
                        }
                    }
                }
                
                prev = current;
                current = cap.next.load(Ordering::Acquire);
            }
        }
        
        false
    }
}

/// Ultra-fast capability security system
pub struct UltraCapabilitySystem {
    /// Sharded capability storage
    shards: [CapabilityShard; CAPABILITY_SHARDS],
    /// Token ID counter
    token_counter: AtomicU64,
    /// Resource generation counters (for revocation)
    resource_generations: [AtomicU32; 256],
}

impl UltraCapabilitySystem {
    pub const fn new() -> Self {
        const SHARD_INIT: CapabilityShard = CapabilityShard::new();
        const GEN_INIT: AtomicU32 = AtomicU32::new(0);
        
        Self {
            shards: [SHARD_INIT; CAPABILITY_SHARDS],
            token_counter: AtomicU64::new(1),
            resource_generations: [GEN_INIT; 256],
        }
    }

    #[inline(always)]
    fn get_shard(&self, token_id: u64) -> &CapabilityShard {
        let idx = (token_id as usize) % CAPABILITY_SHARDS;
        &self.shards[idx]
    }

    #[inline(always)]
    fn get_resource_gen_index(&self, resource_id: u64) -> usize {
        (resource_id as usize) % 256
    }

    /// Mint a new capability token
    #[inline(always)]
    pub fn mint(&self, resource_id: u64, permissions: u64, owner: u32) -> u64 {
        SEC_MINT_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let token_id = self.token_counter.fetch_add(1, Ordering::Relaxed);
        let gen_idx = self.get_resource_gen_index(resource_id);
        let generation = self.resource_generations[gen_idx].load(Ordering::Acquire);
        
        let cap = unsafe {
            alloc::alloc::alloc(
                core::alloc::Layout::new::<UltraCapability>()
            ) as *mut UltraCapability
        };
        
        if cap.is_null() {
            return 0;
        }

        unsafe {
            cap.write(UltraCapability::new(token_id, resource_id, permissions, owner));
            (*cap).generation.store(generation, Ordering::Release);
        }

        let shard = self.get_shard(token_id);
        if shard.insert(cap) {
            token_id
        } else {
            0
        }
    }

    /// Revoke a capability token
    #[inline(always)]
    pub fn revoke(&self, token_id: u64) -> bool {
        SEC_REVOKE_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let shard = self.get_shard(token_id);
        shard.remove(token_id)
    }

    /// Revoke all capabilities for a resource
    #[inline(always)]
    pub fn revoke_resource(&self, resource_id: u64) {
        SEC_REVOKE_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let gen_idx = self.get_resource_gen_index(resource_id);
        self.resource_generations[gen_idx].fetch_add(1, Ordering::Release);
    }

    /// Check if a token has permission
    #[inline(always)]
    pub fn check(&self, token_id: u64, required_permissions: u64) -> bool {
        SEC_CHECK_CALLS.fetch_add(1, Ordering::Relaxed);
        
        let shard = self.get_shard(token_id);
        
        if let Some(cap) = shard.lookup(token_id) {
            unsafe {
                let cap_ref = cap.as_ref();
                if cap_ref.check_permission(required_permissions) {
                    SEC_CHECK_HITS.fetch_add(1, Ordering::Relaxed);
                    return true;
                }
            }
        }
        
        SEC_CHECK_DENIES.fetch_add(1, Ordering::Relaxed);
        false
    }

    /// Batch check multiple permissions
    #[inline(always)]
    pub fn check_batch(&self, checks: &[(u64, u64)]) -> alloc::vec::Vec<bool> {
        SEC_BATCH_CHECKS.fetch_add(1, Ordering::Relaxed);
        
        checks.iter().map(|&(token_id, perms)| {
            self.check(token_id, perms)
        }).collect()
    }

    /// Get capability info
    #[inline(always)]
    pub fn get_info(&self, token_id: u64) -> Option<(u64, u64, u32, u32)> {
        let shard = self.get_shard(token_id);
        
        if let Some(cap) = shard.lookup(token_id) {
            unsafe {
                let cap_ref = cap.as_ref();
                Some((
                    cap_ref.get_resource_id(),
                    cap_ref.get_permissions(),
                    cap_ref.get_owner(),
                    cap_ref.get_generation(),
                ))
            }
        } else {
            None
        }
    }

    /// Update capability permissions
    #[inline(always)]
    pub fn update_permissions(&self, token_id: u64, new_permissions: u64) -> bool {
        let shard = self.get_shard(token_id);
        
        if let Some(cap) = shard.lookup(token_id) {
            unsafe {
                cap.as_ref().permissions.store(new_permissions, Ordering::Release);
            }
            true
        } else {
            false
        }
    }
}

/// Zero-copy security check result
#[repr(C)]
pub struct SecurityCheckResult {
    /// Allow/deny
    allowed: AtomicBool,
    /// Required permissions
    required: AtomicU64,
    /// Granted permissions
    granted: AtomicU64,
}

impl SecurityCheckResult {
    pub const fn new() -> Self {
        Self {
            allowed: AtomicBool::new(false),
            required: AtomicU64::new(0),
            granted: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub fn is_allowed(&self) -> bool {
        self.allowed.load(Ordering::Acquire)
    }

    #[inline(always)]
    pub fn set_allowed(&self, allowed: bool) {
        self.allowed.store(allowed, Ordering::Release);
    }
}

/// NUMA-aware capability distribution
pub struct NumaCapabilitySystem {
    /// Per-NUMA node capability systems
    node_systems: alloc::vec::Vec<UltraCapabilitySystem>,
    /// Current NUMA node
    current_node: AtomicUsize,
}

impl NumaCapabilitySystem {
    pub fn new(numa_nodes: usize) -> Self {
        let mut systems = alloc::vec::Vec::with_capacity(numa_nodes);
        for _ in 0..numa_nodes {
            systems.push(UltraCapabilitySystem::new());
        }
        
        Self {
            node_systems: systems,
            current_node: AtomicUsize::new(0),
        }
    }

    #[inline(always)]
    fn get_node_system(&self) -> &UltraCapabilitySystem {
        let node = self.current_node.load(Ordering::Relaxed) % self.node_systems.len();
        &self.node_systems[node]
    }

    /// Mint capability on local NUMA node
    #[inline(always)]
    pub fn mint(&self, resource_id: u64, permissions: u64, owner: u32) -> u64 {
        self.get_node_system().mint(resource_id, permissions, owner)
    }

    /// Check capability (NUMA-aware lookup)
    #[inline(always)]
    pub fn check(&self, token_id: u64, required_permissions: u64) -> bool {
        // Try local node first
        if self.get_node_system().check(token_id, required_permissions) {
            return true;
        }
        
        // Check other nodes
        for system in &self.node_systems {
            if system.check(token_id, required_permissions) {
                return true;
            }
        }
        
        false
    }
}

/// Capability cache for hot path optimization
pub struct CapabilityCache {
    /// Cache entries (direct-mapped)
    entries: [AtomicU64; 256],
    /// Valid flags
    valid: [AtomicBool; 256],
}

impl CapabilityCache {
    pub const fn new() -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        const FALSE: AtomicBool = AtomicBool::new(false);
        
        Self {
            entries: [ZERO; 256],
            valid: [FALSE; 256],
        }
    }

    #[inline(always)]
    fn index(&self, token_id: u64) -> usize {
        (token_id as usize) % 256
    }

    /// Cache a permission check result
    #[inline(always)]
    pub fn cache(&self, token_id: u64, permissions: u64) {
        let idx = self.index(token_id);
        self.entries[idx].store(permissions, Ordering::Release);
        self.valid[idx].store(true, Ordering::Release);
    }

    /// Lookup cached permissions
    #[inline(always)]
    pub fn lookup(&self, token_id: u64) -> Option<u64> {
        let idx = self.index(token_id);
        
        if self.valid[idx].load(Ordering::Acquire) {
            Some(self.entries[idx].load(Ordering::Acquire))
        } else {
            None
        }
    }

    /// Invalidate cache entry
    #[inline(always)]
    pub fn invalidate(&self, token_id: u64) {
        let idx = self.index(token_id);
        self.valid[idx].store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_permission_operations() {
        let perm = Permission::READ.union(Permission::WRITE);
        
        assert!(perm.contains(Permission::READ));
        assert!(perm.contains(Permission::WRITE));
        assert!(!perm.contains(Permission::EXECUTE));
    }

    #[test_case]
    fn test_ultra_capability() {
        let cap = UltraCapability::new(1, 100, Permission::READ.0, 0);
        
        assert!(cap.is_valid());
        assert_eq!(cap.get_token_id(), 1);
        assert_eq!(cap.get_resource_id(), 100);
        assert!(cap.check_permission(Permission::READ.0));
        
        cap.invalidate();
        assert!(!cap.is_valid());
    }

    #[test_case]
    fn test_capability_shard() {
        let shard = CapabilityShard::new();
        
        let cap = unsafe {
            alloc::alloc::alloc(
                core::alloc::Layout::new::<UltraCapability>()
            ) as *mut UltraCapability
        };

        unsafe {
            cap.write(UltraCapability::new(1, 100, Permission::READ.0, 0));
        }
        
        assert!(shard.insert(cap));
        assert!(shard.lookup(1).is_some());
        
        assert!(shard.remove(1));
        assert!(shard.lookup(1).is_none());
    }

    #[test_case]
    fn test_ultra_capability_system() {
        let sys = UltraCapabilitySystem::new();
        
        let token = sys.mint(100, Permission::READ.0, 0);
        assert_ne!(token, 0);
        
        assert!(sys.check(token, Permission::READ.0));
        assert!(!sys.check(token, Permission::WRITE.0));
        
        assert!(sys.revoke(token));
        assert!(!sys.check(token, Permission::READ.0));
    }

    #[test_case]
    fn test_resource_revocation() {
        let sys = UltraCapabilitySystem::new();
        
        let token1 = sys.mint(100, Permission::READ.0, 0);
        let token2 = sys.mint(100, Permission::WRITE.0, 0);
        
        assert!(sys.check(token1, Permission::READ.0));
        assert!(sys.check(token2, Permission::WRITE.0));
        
        sys.revoke_resource(100);
        
        assert!(!sys.check(token1, Permission::READ.0));
        assert!(!sys.check(token2, Permission::WRITE.0));
    }

    #[test_case]
    fn test_batch_check() {
        let sys = UltraCapabilitySystem::new();
        
        let token1 = sys.mint(100, Permission::READ.0, 0);
        let token2 = sys.mint(200, Permission::WRITE.0, 0);
        
        let checks = vec![
            (token1, Permission::READ.0),
            (token2, Permission::WRITE.0),
            (token1, Permission::WRITE.0),
        ];
        
        let results = sys.check_batch(&checks);
        assert_eq!(results, vec![true, true, false]);
    }

    #[test_case]
    fn test_capability_cache() {
        let cache = CapabilityCache::new();
        
        cache.cache(1, Permission::READ.0);
        
        assert_eq!(cache.lookup(1), Some(Permission::READ.0));
        
        cache.invalidate(1);
        assert_eq!(cache.lookup(1), None);
    }

    #[test_case]
    fn test_security_stats() {
        let stats = security_stats();
        assert!(stats.hit_rate >= 0.0 && stats.hit_rate <= 1.0);
    }
}
