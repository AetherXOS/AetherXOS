use crate::interfaces::security::{
    cap_flags, ResourceKind, SecurityAction, SecurityContext, SecurityVerdict,
};
use crate::interfaces::task::TaskId;
use crate::interfaces::SecurityMonitor;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use aethercore_common::{counter_inc, counter_load, declare_counter_u64, const_assert};
use aethercore_common::telemetry;

// ─── Telemetry Counters ─────────────────────────────────────────────
declare_counter_u64!(CAP_MINT_CALLS);
declare_counter_u64!(CAP_REVOKE_CALLS);
declare_counter_u64!(CAP_ACCESS_CALLS);
declare_counter_u64!(CAP_ACCESS_HITS);
declare_counter_u64!(CAP_ACCESS_DENIED);
declare_counter_u64!(CAP_DELEGATE_CALLS);
declare_counter_u64!(CAP_FULL_ACCESS_CALLS);
static CAP_TOKEN_COUNTER: AtomicU64 = AtomicU64::new(1);

const TOKEN_MIX_1: u64 = 0x9E3779B97F4A7C15;
const TOKEN_MIX_2: u64 = 0x517CC1B727220A95;
const TOKEN_MIX_3: u64 = 0x6C62272E07BB0142;
const TOKEN_SHIFT_1: u32 = 32;
const TOKEN_SHIFT_2: u32 = 28;

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

/// Pseudo-random token generation using a simple PRNG seeded by counter + resource.
/// Not cryptographic, but non-guessable enough for an in-kernel capability model.
fn generate_token_id(resource_id: u64) -> u64 {
    let counter = CAP_TOKEN_COUNTER.fetch_add(1, Ordering::Relaxed);
    // Mix bits using xorshift-like transform
    let mut h = counter.wrapping_mul(TOKEN_MIX_1);
    h ^= resource_id;
    h = h.wrapping_mul(TOKEN_MIX_2);
    h ^= h >> TOKEN_SHIFT_1;
    h = h.wrapping_mul(TOKEN_MIX_3);
    h ^= h >> TOKEN_SHIFT_2;
    // Ensure non-zero
    if h == 0 {
        h = 1;
    }
    h
}

// ─── Object Capability Monitor ──────────────────────────────────────

/// Object Capability Monitor — Production Grade.
///
/// Access is determined by possession of a capability token with
/// matching resource ID and sufficient permission bits.
pub struct ObjectCapability {
    /// token_id -> CapabilityToken
    tokens: Mutex<BTreeMap<u64, CapabilityToken>>,
    /// resource_id -> current generation (revoked tokens have stale gen)
    generations: Mutex<BTreeMap<u64, u64>>,
}

impl ObjectCapability {
    pub const fn new() -> Self {
        Self {
            tokens: Mutex::new(BTreeMap::new()),
            generations: Mutex::new(BTreeMap::new()),
        }
    }

    /// Mint a token for a resource with specific permissions.
    pub fn mint_token(&self, resource_id: u64) -> u64 {
        self.mint_token_for(resource_id, TaskId(0), PERM_ALL, true)
    }

    /// Mint a token with full control over permissions, ownership and delegation.
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

        self.tokens.lock().insert(token_id, token);
        token_id
    }

    /// Revoke a specific token.
    pub fn revoke_token(&self, token_id: u64) -> bool {
        counter_inc!(CAP_REVOKE_CALLS);
        self.tokens.lock().remove(&token_id).is_some()
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
            let tokens = self.tokens.lock();
            let original = tokens.get(&token_id)?;

            if !original.delegatable {
                return None;
            }

            let perms = match restricted_perms {
                Some(mask) => original.permissions & mask, // Can only narrow, not widen
                None => original.permissions,
            };

            (original.resource_id, perms)
        }; // lock released here

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
        self.tokens.lock().len()
    }
}

impl SecurityMonitor for ObjectCapability {
    fn check_access(&self, resource_handle: u64) -> bool {
        CAP_ACCESS_CALLS.fetch_add(1, Ordering::Relaxed);
        let tokens = self.tokens.lock();
        if let Some(token) = tokens.get(&resource_handle) {
            if self.is_token_valid(token) {
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
        let tokens = self.tokens.lock();
        for token in tokens.values() {
            if token.resource_id == resource_id
                && token.owner == ctx.task_id
                && (token.permissions & required_perm) == required_perm
                && self.is_token_valid(token)
            {
                CAP_ACCESS_HITS.fetch_add(1, Ordering::Relaxed);
                return SecurityVerdict::Allow;
            }
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
        let tokens = cap.tokens.lock();
        let del_token = tokens.get(&del_id).unwrap();
        assert_eq!(del_token.permissions, PERM_READ);
        assert_eq!(del_token.owner, delegate);
        assert!(!del_token.delegatable);
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
