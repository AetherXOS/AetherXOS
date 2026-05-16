use super::*;
use crate::interfaces::task::TaskId;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

#[derive(Clone)]
pub struct SeccompFilter {
    // In a real implementation, this would be a BPF program.
    // For now, we use a simple "allowed syscalls" set or a stub.
    pub allowed_nr: alloc::collections::BTreeSet<usize>,
}

lazy_static! {
    static ref TASK_FILTERS: Mutex<BTreeMap<TaskId, SeccompFilter>> = Mutex::new(BTreeMap::new());
}

pub fn set_filter(task_id: TaskId, filter: SeccompFilter) {
    TASK_FILTERS.lock().insert(task_id, filter);
}

pub fn check_seccomp_policy(nr: usize, _frame: &crate::modules::linux_compat::sys_dispatcher::SyscallDispFrame) -> Option<usize> {
    let tid = TaskId(crate::modules::posix::process::gettid());
    let map = TASK_FILTERS.lock();
    let filter = match map.get(&tid) {
        Some(f) => f,
        None => return None, // No filter
    };

    if filter.allowed_nr.contains(&nr) {
        None // Allowed
    } else {
        // Denied: Return EPERM or kill process?
        // Linux seccomp default is SIGSYS or kill thread.
        // For now, return EPERM.
        Some(linux_errno(crate::modules::posix_consts::errno::EPERM))
    }
}

pub fn sys_linux_seccomp(operation: usize, _flags: usize, _args: UserPtr<u8>) -> usize {
    const SECCOMP_SET_MODE_STRICT: usize = 0;
    const SECCOMP_SET_MODE_FILTER: usize = 1;

    match operation {
        SECCOMP_SET_MODE_STRICT => {
            let tid = TaskId(crate::modules::posix::process::gettid());
            let mut filter = SeccompFilter { allowed_nr: alloc::collections::BTreeSet::new() };
            // Strict mode only allows read, write, exit, sigreturn
            filter.allowed_nr.insert(0); // READ
            filter.allowed_nr.insert(1); // WRITE
            filter.allowed_nr.insert(60); // EXIT
            filter.allowed_nr.insert(15); // RT_SIGRETURN
            set_filter(tid, filter);
            0
        }
        SECCOMP_SET_MODE_FILTER => {
            // In a real implementation, we would parse the BPF from args.
            // For now, we allow everything but log the intent.
            crate::klog_info!("[SECCOMP] Task {} requested BPF filter (stubbed as AllowAll)", crate::modules::posix::process::gettid());
            0
        }
        _ => linux_errno(crate::modules::posix_consts::errno::EINVAL),
    }
}
