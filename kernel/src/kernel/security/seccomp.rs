use alloc::collections::BTreeSet;
use spin::Mutex;
use crate::interfaces::TaskId;

/// Seccomp Filter: Restricts which syscalls a task can perform.
#[derive(Debug, Clone)]
pub struct SeccompFilter {
    /// Set of allowed syscall numbers.
    allowed_syscalls: BTreeSet<u32>,
    /// Action to take when a forbidden syscall is attempted.
    on_violation: SeccompAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAction {
    Kill,
    Trap,
    Errno(u32),
}

impl SeccompFilter {
    pub fn new(allowed: BTreeSet<u32>, action: SeccompAction) -> Self {
        Self {
            allowed_syscalls: allowed,
            on_violation: action,
        }
    }

    /// Check if a syscall is allowed.
    pub fn is_allowed(&self, syscall_nr: u32) -> bool {
        self.allowed_syscalls.contains(&syscall_nr)
    }

    pub fn violation_action(&self) -> SeccompAction {
        self.on_violation
    }
}

lazy_static::lazy_static! {
    /// Global registry of per-task seccomp filters.
    pub static ref SECCOMP_REGISTRY: Mutex<alloc::collections::BTreeMap<TaskId, SeccompFilter>> =
        Mutex::new(alloc::collections::BTreeMap::new());
}

pub fn apply_filter(tid: TaskId, filter: SeccompFilter) {
    SECCOMP_REGISTRY.lock().insert(tid, filter);
}

pub fn check_syscall(tid: TaskId, nr: u32) -> Result<(), SeccompAction> {
    let registry = SECCOMP_REGISTRY.lock();
    if let Some(filter) = registry.get(&tid) {
        if filter.is_allowed(nr) {
            Ok(())
        } else {
            Err(filter.violation_action())
        }
    } else {
        Ok(()) // No filter applied
    }
}
