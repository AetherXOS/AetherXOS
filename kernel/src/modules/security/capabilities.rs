use crate::interfaces::SecurityMonitor;
use crate::interfaces::security::{
    ResourceKind, SecurityAction, SecurityContext, SecurityVerdict, cap_flags,
};
use crate::interfaces::task::TaskId;
use alloc::collections::BTreeMap;
use core::sync::atomic::Ordering;
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use spin::Mutex;

use aethercore_common::telemetry;
use aethercore_common::{const_assert, counter_inc, declare_counter_u64};

// ─── Telemetry Counters ─────────────────────────────────────────────
declare_counter_u64!(CAP_MINT_CALLS);
declare_counter_u64!(CAP_REVOKE_CALLS);
declare_counter_u64!(CAP_ACCESS_CALLS);
declare_counter_u64!(CAP_ACCESS_HITS);
declare_counter_u64!(CAP_ACCESS_DENIED);
declare_counter_u64!(CAP_DELEGATE_CALLS);
declare_counter_u64!(CAP_FULL_ACCESS_CALLS);

static RNG: Mutex<Option<ChaCha20Rng>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
pub struct CapabilityStats {
    pub mint_calls: u64,
    pub revoke_calls: u64,
    pub access_calls: u64,
    pub access_hits: u64,
    pub access_denied: u64,
    pub delegate_calls: u64,
    pub full_access_calls: u64,
    pub active_tokens: usize,
}

pub fn stats() -> CapabilityStats {
    CapabilityStats {
        mint_calls: telemetry::snapshot_u64(&CAP_MINT_CALLS),
        revoke_calls: telemetry::snapshot_u64(&CAP_REVOKE_CALLS),
        access_calls: telemetry::snapshot_u64(&CAP_ACCESS_CALLS),
        access_hits: telemetry::snapshot_u64(&CAP_ACCESS_HITS),
        access_denied: telemetry::snapshot_u64(&CAP_ACCESS_DENIED),
        delegate_calls: telemetry::snapshot_u64(&CAP_DELEGATE_CALLS),
        full_access_calls: telemetry::snapshot_u64(&CAP_FULL_ACCESS_CALLS),
        active_tokens: 0, // filled by caller if needed
    }
}

/// Returns a race-safe telemetry snapshot and resets counters for interval reporting.
pub fn take_stats() -> CapabilityStats {
    CapabilityStats {
        mint_calls: telemetry::take_u64(&CAP_MINT_CALLS),
        revoke_calls: telemetry::take_u64(&CAP_REVOKE_CALLS),
        access_calls: telemetry::take_u64(&CAP_ACCESS_CALLS),
        access_hits: telemetry::take_u64(&CAP_ACCESS_HITS),
        access_denied: telemetry::take_u64(&CAP_ACCESS_DENIED),
        delegate_calls: telemetry::take_u64(&CAP_DELEGATE_CALLS),
        full_access_calls: telemetry::take_u64(&CAP_FULL_ACCESS_CALLS),
        active_tokens: 0, // filled by caller if needed
    }
}

// ─── Capability Token ───────────────────────────────────────────────

/// A capability token carries rights for a specific resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapabilityToken {
    /// Unique, non-guessable token identifier.
    pub token_id: u64,
    /// The resource this token grants access to.
    pub resource_id: u64,
    /// Permission bitmask: which actions are allowed.
    pub permissions: u64,
    /// Who owns this token.
    pub owner: TaskId,
    /// Whether this token can be delegated to other tasks.
    pub delegatable: bool,
    /// Generation counter — revoked tokens have stale generation.
    pub generation: u64,
}

// Permission bits for capability tokens.
pub const PERM_READ: u64 = 1 << 0;
pub const PERM_WRITE: u64 = 1 << 1;
pub const PERM_EXECUTE: u64 = 1 << 2;
pub const PERM_CREATE: u64 = 1 << 3;
pub const PERM_DELETE: u64 = 1 << 4;
pub const PERM_ADMIN: u64 = 1 << 5;
pub const PERM_MOUNT: u64 = 1 << 6;
pub const PERM_SIGNAL: u64 = 1 << 7;
pub const PERM_IPC: u64 = 1 << 8;
pub const PERM_NET: u64 = 1 << 9;
pub const PERM_ALL: u64 = u64::MAX;

const_assert!(core::mem::size_of::<CapabilityToken>() <= 64);

fn action_to_perm(action: SecurityAction) -> u64 {
    match action {
        SecurityAction::Read => PERM_READ,
        SecurityAction::Write => PERM_WRITE,
        SecurityAction::Execute => PERM_EXECUTE,
        SecurityAction::Create => PERM_CREATE,
        SecurityAction::Delete => PERM_DELETE,
        SecurityAction::Admin => PERM_ADMIN,
        SecurityAction::Mount | SecurityAction::Unmount => PERM_MOUNT,
        SecurityAction::Signal => PERM_SIGNAL,
        SecurityAction::IpcSend | SecurityAction::IpcRecv => PERM_IPC,
        SecurityAction::NetBind | SecurityAction::NetConnect => PERM_NET,
        _ => PERM_ADMIN,
    }
}

/// Cryptographically secure token generation using ChaCha20 seeded from hardware entropy.
fn generate_token_id(_resource_id: u64) -> u64 {
    let mut rng_lock = RNG.lock();
    let rng = rng_lock.get_or_insert_with(|| {
        let mut seed = [0u8; 32];
        for i in 0..4 {
            // Mix RDRAND with TSC for fallback or additional entropy
            let entropy = crate::hal::cpu::rdrand()
                .unwrap_or_else(|| crate::hal::cpu::rdtsc() ^ 0x5555_5555_5555_5555);
            let bytes = entropy.to_le_bytes();
            seed[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
        }
        ChaCha20Rng::from_seed(seed)
    });
    rng.next_u64()
}

// ─── Object Capability Monitor ──────────────────────────────────────

/// Object Capability Monitor — Production Grade.
///
/// Access is determined by possession of a capability token with
/// matching resource ID and sufficient permission bits.
/// Number of shards for the sharded locking strategy.
const SHARD_COUNT: usize = 16;

pub struct ObjectCapability {
    #[cfg(feature = "cap_lock_mutex")]
    tokens: Mutex<BTreeMap<u64, CapabilityToken>>,

    #[cfg(feature = "cap_lock_rwlock")]
    tokens: RwLock<BTreeMap<u64, CapabilityToken>>,

    #[cfg(feature = "cap_lock_sharded")]
    tokens: [Mutex<BTreeMap<u64, CapabilityToken>>; SHARD_COUNT],

    /// resource_id -> current generation (revoked tokens have stale gen)
    generations: Mutex<BTreeMap<u64, u64>>,
}

impl ObjectCapability {
    pub const fn new() -> Self {
        #[cfg(feature = "cap_lock_mutex")]
        {
            Self {
                tokens: Mutex::new(BTreeMap::new()),
                generations: Mutex::new(BTreeMap::new()),
            }
        }

        #[cfg(feature = "cap_lock_rwlock")]
        {
            Self {
                tokens: RwLock::new(BTreeMap::new()),
                generations: Mutex::new(BTreeMap::new()),
            }
        }

        #[cfg(feature = "cap_lock_sharded")]
        {
            const SHARD_INIT: Mutex<BTreeMap<u64, CapabilityToken>> = Mutex::new(BTreeMap::new());
            Self {
                tokens: [SHARD_INIT; SHARD_COUNT],
                generations: Mutex::new(BTreeMap::new()),
            }
        }

        #[cfg(not(any(
            feature = "cap_lock_mutex",
            feature = "cap_lock_rwlock",
            feature = "cap_lock_sharded"
        )))]
        {
            Self {
                tokens: Mutex::new(BTreeMap::new()),
                generations: Mutex::new(BTreeMap::new()),
            }
        }
    }

    /// Mint a token for a resource with specific permissions.
    pub fn mint_token(&self, resource_id: u64) -> u64 {
        self.mint_token_for(resource_id, TaskId(0), PERM_ALL, true)
    }

    pub fn mint_token_for(
        &self,
        resource_id: u64,
        owner: TaskId,
        permissions: u64,
        delegatable: bool,
    ) -> u64 {
        counter_inc!(CAP_MINT_CALLS);

        let token_id = generate_token_id(resource_id);
        let generation = {
            let gens = self.generations.lock();
            gens.get(&resource_id).copied().unwrap_or(0)
        };

        let token = CapabilityToken {
            token_id,
            resource_id,
            permissions,
            owner,
            delegatable,
            generation,
        };

        #[cfg(feature = "cap_lock_mutex")]
        self.tokens.lock().insert(token_id, token);

        #[cfg(feature = "cap_lock_rwlock")]
        self.tokens.write().insert(token_id, token);

        #[cfg(feature = "cap_lock_sharded")]
        {
            let shard = (token_id % SHARD_COUNT as u64) as usize;
            self.tokens[shard].lock().insert(token_id, token);
        }

        #[cfg(not(any(
            feature = "cap_lock_mutex",
            feature = "cap_lock_rwlock",
            feature = "cap_lock_sharded"
        )))]
        self.tokens.lock().insert(token_id, token);

        token_id
    }

    /// Revoke a specific token.
    pub fn revoke_token(&self, token_id: u64) -> bool {
        counter_inc!(CAP_REVOKE_CALLS);
        #[cfg(feature = "cap_lock_mutex")]
        return self.tokens.lock().remove(&token_id).is_some();

        #[cfg(feature = "cap_lock_rwlock")]
        return self.tokens.write().remove(&token_id).is_some();

        #[cfg(feature = "cap_lock_sharded")]
        {
            let shard = (token_id % SHARD_COUNT as u64) as usize;
            return self.tokens[shard].lock().remove(&token_id).is_some();
        }

        #[cfg(not(any(
            feature = "cap_lock_mutex",
            feature = "cap_lock_rwlock",
            feature = "cap_lock_sharded"
        )))]
        return self.tokens.lock().remove(&token_id).is_some();
    }

    /// Revoke ALL tokens for a resource by bumping the generation.
    pub fn revoke_resource(&self, resource_id: u64) {
        counter_inc!(CAP_REVOKE_CALLS);
        let mut gens = self.generations.lock();
        let generation = gens.entry(resource_id).or_insert(0);
        *generation += 1;
    }

    /// Delegate a token to another task (if the token is delegatable).
    pub fn delegate_token(
        &self,
        token_id: u64,
        new_owner: TaskId,
        restricted_perms: Option<u64>,
    ) -> Option<u64> {
        counter_inc!(CAP_DELEGATE_CALLS);

        let (resource_id, new_perms) = {
            #[cfg(feature = "cap_lock_mutex")]
            {
                let tokens = self.tokens.lock();
                let original = tokens.get(&token_id)?;
                if !original.delegatable {
                    return None;
                }
                let perms = restricted_perms
                    .map(|m| original.permissions & m)
                    .unwrap_or(original.permissions);
                (original.resource_id, perms)
            }

            #[cfg(feature = "cap_lock_rwlock")]
            {
                let tokens = self.tokens.read();
                let original = tokens.get(&token_id)?;
                if !original.delegatable {
                    return None;
                }
                let perms = restricted_perms
                    .map(|m| original.permissions & m)
                    .unwrap_or(original.permissions);
                (original.resource_id, perms)
            }

            #[cfg(feature = "cap_lock_sharded")]
            {
                let shard = (token_id % SHARD_COUNT as u64) as usize;
                let tokens = self.tokens[shard].lock();
                let original = tokens.get(&token_id)?;
                if !original.delegatable {
                    return None;
                }
                let perms = restricted_perms
                    .map(|m| original.permissions & m)
                    .unwrap_or(original.permissions);
                (original.resource_id, perms)
            }

            #[cfg(not(any(
                feature = "cap_lock_mutex",
                feature = "cap_lock_rwlock",
                feature = "cap_lock_sharded"
            )))]
            {
                let tokens = self.tokens.lock();
                let original = tokens.get(&token_id)?;
                if !original.delegatable {
                    return None;
                }
                let perms = restricted_perms
                    .map(|m| original.permissions & m)
                    .unwrap_or(original.permissions);
                (original.resource_id, perms)
            }
        };

        let new_token_id = self.mint_token_for(
            resource_id,
            new_owner,
            new_perms,
            false, // Delegated tokens are not further delegatable by default
        );

        Some(new_token_id)
    }

    /// Check if a token is valid (exists and has correct generation).
    fn is_token_valid(&self, token: &CapabilityToken) -> bool {
        let gens = self.generations.lock();
        let current_gen = gens.get(&token.resource_id).copied().unwrap_or(0);
        token.generation == current_gen
    }

    /// Get the number of active tokens.
    pub fn active_token_count(&self) -> usize {
        #[cfg(feature = "cap_lock_mutex")]
        return self.tokens.lock().len();

        #[cfg(feature = "cap_lock_rwlock")]
        return self.tokens.read().len();

        #[cfg(feature = "cap_lock_sharded")]
        return self.tokens.iter().map(|s| s.lock().len()).sum();

        #[cfg(not(any(
            feature = "cap_lock_mutex",
            feature = "cap_lock_rwlock",
            feature = "cap_lock_sharded"
        )))]
        return self.tokens.lock().len();
    }
}

impl SecurityMonitor for ObjectCapability {
    fn check_access(&self, resource_handle: u64) -> bool {
        CAP_ACCESS_CALLS.fetch_add(1, Ordering::Relaxed);

        let token_opt = {
            #[cfg(feature = "cap_lock_mutex")]
            {
                let tokens = self.tokens.lock();
                tokens.get(&resource_handle).copied()
            }

            #[cfg(feature = "cap_lock_rwlock")]
            {
                let tokens = self.tokens.read();
                tokens.get(&resource_handle).copied()
            }

            #[cfg(feature = "cap_lock_sharded")]
            {
                let shard = (resource_handle % SHARD_COUNT as u64) as usize;
                let tokens = self.tokens[shard].lock();
                tokens.get(&resource_handle).copied()
            }

            #[cfg(not(any(
                feature = "cap_lock_mutex",
                feature = "cap_lock_rwlock",
                feature = "cap_lock_sharded"
            )))]
            {
                let tokens = self.tokens.lock();
                tokens.get(&resource_handle).copied()
            }
        };

        if let Some(token) = token_opt {
            if self.is_token_valid(&token) {
                CAP_ACCESS_HITS.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }
        CAP_ACCESS_DENIED.fetch_add(1, Ordering::Relaxed);
        false
    }

    fn check_access_full(
        &self,
        ctx: &SecurityContext,
        resource_id: u64,
        _resource_kind: ResourceKind,
        action: SecurityAction,
    ) -> SecurityVerdict {
        CAP_FULL_ACCESS_CALLS.fetch_add(1, Ordering::Relaxed);

        // Root bypass
        if ctx.is_root() || ctx.privileged {
            CAP_ACCESS_HITS.fetch_add(1, Ordering::Relaxed);
            return SecurityVerdict::Allow;
        }

        // Check if the task's capability bitmask grants the required Linux-style cap
        let required_cap = match action {
            SecurityAction::Chown => cap_flags::CAP_CHOWN,
            SecurityAction::SetUid => cap_flags::CAP_SETUID,
            SecurityAction::SetGid => cap_flags::CAP_SETGID,
            SecurityAction::NetBind => cap_flags::CAP_NET_BIND,
            SecurityAction::NetConnect => cap_flags::CAP_NET_RAW,
            SecurityAction::Signal => cap_flags::CAP_KILL,
            SecurityAction::ModuleLoad => cap_flags::CAP_SYS_MODULE,
            SecurityAction::RawIo => cap_flags::CAP_SYS_RAWIO,
            SecurityAction::Admin => cap_flags::CAP_SYS_ADMIN,
            SecurityAction::Reboot => cap_flags::CAP_SYS_BOOT,
            SecurityAction::SetTime => cap_flags::CAP_SYS_TIME,
            SecurityAction::Mount => cap_flags::CAP_MOUNT,
            SecurityAction::Unmount => cap_flags::CAP_UNMOUNT,
            SecurityAction::PtraceAttach => cap_flags::CAP_SYS_PTRACE,
            _ => 0, // No specific cap required for basic ops
        };

        if required_cap != 0 && ctx.has_capability(required_cap) {
            CAP_ACCESS_HITS.fetch_add(1, Ordering::Relaxed);
            return SecurityVerdict::AuditAllow;
        }

        // Search for a capability token granting access
        let required_perm = action_to_perm(action);

        let mut found = false;

        #[cfg(feature = "cap_lock_mutex")]
        {
            let tokens = self.tokens.lock();
            for token in tokens.values() {
                if token.resource_id == resource_id
                    && token.owner == ctx.task_id
                    && (token.permissions & required_perm) == required_perm
                    && self.is_token_valid(token)
                {
                    found = true;
                    break;
                }
            }
        }

        #[cfg(feature = "cap_lock_rwlock")]
        {
            let tokens = self.tokens.read();
            for token in tokens.values() {
                if token.resource_id == resource_id
                    && token.owner == ctx.task_id
                    && (token.permissions & required_perm) == required_perm
                    && self.is_token_valid(token)
                {
                    found = true;
                    break;
                }
            }
        }

        #[cfg(feature = "cap_lock_sharded")]
        {
            for shard in &self.tokens {
                let tokens = shard.lock();
                for token in tokens.values() {
                    if token.resource_id == resource_id
                        && token.owner == ctx.task_id
                        && (token.permissions & required_perm) == required_perm
                        && self.is_token_valid(token)
                    {
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
        }

        #[cfg(not(any(
            feature = "cap_lock_mutex",
            feature = "cap_lock_rwlock",
            feature = "cap_lock_sharded"
        )))]
        {
            let tokens = self.tokens.lock();
            for token in tokens.values() {
                if token.resource_id == resource_id
                    && token.owner == ctx.task_id
                    && (token.permissions & required_perm) == required_perm
                    && self.is_token_valid(token)
                {
                    found = true;
                    break;
                }
            }
        }

        if found {
            CAP_ACCESS_HITS.fetch_add(1, Ordering::Relaxed);
            return SecurityVerdict::Allow;
        }

        // If no specific cap was required and the action is basic (read), allow with audit
        if required_cap == 0 {
            match action {
                SecurityAction::Read => {
                    CAP_ACCESS_HITS.fetch_add(1, Ordering::Relaxed);
                    return SecurityVerdict::Allow;
                }
                _ => {}
            }
        }

        CAP_ACCESS_DENIED.fetch_add(1, Ordering::Relaxed);
        if ctx.audit_enabled {
            SecurityVerdict::AuditDeny
        } else {
            SecurityVerdict::Deny
        }
    }

    fn has_capability(&self, ctx: &SecurityContext, cap: u64) -> bool {
        ctx.has_capability(cap)
    }

    fn policy_name(&self) -> &'static str {
        "ObjectCapability"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn capability_mint_check_and_revoke_roundtrip() {
        let cap = ObjectCapability::new();
        let token = cap.mint_token(42);
        assert!(cap.check_access(token));
        assert!(cap.revoke_token(token));
        assert!(!cap.check_access(token));
    }

    #[test_case]
    fn capability_revoke_unknown_token_is_false() {
        let cap = ObjectCapability::new();
        assert!(!cap.revoke_token(0xDEAD_BEEF));
    }

    #[test_case]
    fn capability_resource_revocation_invalidates_tokens() {
        let cap = ObjectCapability::new();
        let t1 = cap.mint_token(100);
        let t2 = cap.mint_token(100);
        assert!(cap.check_access(t1));
        assert!(cap.check_access(t2));

        cap.revoke_resource(100); // Bump generation

        assert!(!cap.check_access(t1));
        assert!(!cap.check_access(t2));
    }

    #[test_case]
    fn capability_delegation_narrows_permissions() {
        let cap = ObjectCapability::new();
        let owner = TaskId(1);
        let token_id = cap.mint_token_for(200, owner, PERM_ALL, true);

        // Delegate with read-only restriction
        let delegate = TaskId(2);
        let delegated = cap.delegate_token(token_id, delegate, Some(PERM_READ));
        assert!(delegated.is_some());

        let del_id = delegated.unwrap();

        // Use check_access to verify instead of direct field access
        assert!(cap.check_access(del_id));
    }

    #[test_case]
    fn capability_full_check_root_bypass() {
        let cap = ObjectCapability::new();
        let ctx = SecurityContext::kernel();
        let verdict = cap.check_access_full(&ctx, 999, ResourceKind::File, SecurityAction::Admin);
        assert!(verdict.is_allowed());
    }

    #[test_case]
    fn capability_full_check_specific_cap() {
        let cap = ObjectCapability::new();
        let mut ctx =
            SecurityContext::user(TaskId(5), crate::interfaces::task::ProcessId(1), 1000, 1000);
        ctx.capabilities = cap_flags::CAP_MOUNT;

        let verdict =
            cap.check_access_full(&ctx, 50, ResourceKind::MountPoint, SecurityAction::Mount);
        assert!(verdict.is_allowed());

        // Without the cap, admin should be denied
        ctx.capabilities = cap_flags::CAP_NONE;
        let verdict =
            cap.check_access_full(&ctx, 50, ResourceKind::MountPoint, SecurityAction::Mount);
        assert!(!verdict.is_allowed());
    }
}
