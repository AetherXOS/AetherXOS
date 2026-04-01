use super::super::*;
use crate::interfaces::task::{CpuId, TaskId};

// Maximum CPUs supported in cpu_set_t (linux: 1024 typically, kernel usually 8-256).
const MAX_CPUS: usize = 256;
const MAX_AFFINITY_CPUS: usize = 64;
const SCHED_OTHER: usize = crate::modules::posix_consts::process::SCHED_OTHER as usize;
const SCHED_FIFO: usize = crate::modules::posix_consts::process::SCHED_FIFO as usize;
const SCHED_RR: usize = crate::modules::posix_consts::process::SCHED_RR as usize;
const DEFAULT_NORMAL_PRIORITY: u8 = 128;
const LINUX_RT_PRIO_MAX: u32 = 99;

#[inline(always)]
fn resolve_target_task(pid: usize) -> Result<TaskId, usize> {
    if pid == 0 {
        Ok(unsafe { crate::kernel::cpu_local::CpuLocal::get().current_task_id() })
    } else {
        Ok(TaskId(pid))
    }
}

#[inline(always)]
fn is_supported_policy(policy: usize) -> bool {
    matches!(policy, SCHED_OTHER | SCHED_FIFO | SCHED_RR)
}

#[inline(always)]
fn policy_priority_bounds(policy: usize) -> Option<(u32, u32)> {
    match policy {
        SCHED_OTHER => Some((0, 0)),
        SCHED_FIFO | SCHED_RR => Some((1, LINUX_RT_PRIO_MAX)),
        _ => None,
    }
}

#[inline(always)]
fn linux_rt_to_internal_priority(prio: u32) -> u8 {
    // Linux RT prio 99 should map to highest internal priority (0).
    let p = prio.clamp(1, LINUX_RT_PRIO_MAX);
    let scaled = p.saturating_mul(255) / LINUX_RT_PRIO_MAX;
    255u8.saturating_sub(scaled as u8)
}

#[inline(always)]
fn internal_to_linux_rt_priority(priority: u8) -> u32 {
    let inv = 255u32.saturating_sub(priority as u32);
    let scaled = inv.saturating_mul(LINUX_RT_PRIO_MAX) / 255;
    scaled.max(1)
}

/// `sched_getaffinity(2)` — returns a cpumask reflecting the actual number of
/// CPUs present on the system, not just a hardcoded 0xFF fill.
pub fn sys_linux_sched_getaffinity(pid: usize, cpusetsize: usize, mask: UserPtr<u8>) -> usize {
    if mask.is_null() || cpusetsize == 0 {
        return linux_inval();
    }

    // Get real CPU count from smp layer.
    let ncpus = crate::hal::smp::cpu_count().min(MAX_CPUS);
    let bytes_needed = (ncpus + 7) / 8;
    if cpusetsize < bytes_needed {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    let target = match resolve_target_task(pid) {
        Ok(tid) => tid,
        Err(errno) => return errno,
    };
    let Some(task_arc) = crate::kernel::task::get_task(target) else {
        return linux_errno(crate::modules::posix_consts::errno::ESRCH);
    };

    let task_mask = task_arc.lock().cpu_affinity_mask;
    let max_bits = ncpus.min(MAX_AFFINITY_CPUS);
    let online_mask = if max_bits >= 64 {
        u64::MAX
    } else {
        (1u64 << max_bits) - 1
    };
    let effective_mask = task_mask & online_mask;

    if with_user_write_bytes(mask.addr, cpusetsize, |dst| {
        dst.fill(0);
        for cpu in 0..max_bits {
            if (effective_mask & (1u64 << cpu)) != 0 {
                dst[cpu / 8] |= 1 << (cpu % 8);
            }
        }
        0
    })
    .is_err()
    {
        return linux_fault();
    }

    bytes_needed
}

/// `sched_setaffinity(2)` — store the affinity hint on the current task.
pub fn sys_linux_sched_setaffinity(pid: usize, cpusetsize: usize, mask_ptr: UserPtr<u8>) -> usize {
    if mask_ptr.is_null() || cpusetsize == 0 {
        return linux_inval();
    }

    let target = match resolve_target_task(pid) {
        Ok(tid) => tid,
        Err(errno) => return errno,
    };
    let Some(task_arc) = crate::kernel::task::get_task(target) else {
        return linux_errno(crate::modules::posix_consts::errno::ESRCH);
    };

    let ncpus = crate::hal::smp::cpu_count().min(MAX_CPUS);
    let max_bits = ncpus.min(MAX_AFFINITY_CPUS);
    let raw_mask = match with_user_read_bytes(mask_ptr.addr, cpusetsize, |src| {
        let mut mask = 0u64;
        for cpu in 0..max_bits {
            let byte_index = cpu / 8;
            if byte_index >= src.len() {
                break;
            }
            if (src[byte_index] & (1 << (cpu % 8))) != 0 {
                mask |= 1u64 << cpu;
            }
        }
        mask
    }) {
        Ok(m) => m,
        Err(_) => return linux_fault(),
    };

    if raw_mask == 0 {
        return linux_errno(crate::modules::posix_consts::errno::EINVAL);
    }

    {
        let mut task = task_arc.lock();
        task.cpu_affinity_mask = raw_mask;
        let preferred = raw_mask.trailing_zeros() as usize;
        task.preferred_cpu = CpuId(preferred);
    }

    0
}

/// `sched_getparam(2)` / `sched_getscheduler(2)` / `sched_setscheduler(2)` stubs.
pub fn sys_linux_sched_getscheduler(pid: usize) -> usize {
    let target = if pid == 0 {
        unsafe { crate::kernel::cpu_local::CpuLocal::get().current_task_id() }
    } else {
        crate::interfaces::task::TaskId(pid)
    };

    if let Some(task_arc) = crate::kernel::task::get_task(target) {
        let task = task_arc.lock();
        if task.rt_group_id > 0 {
            SCHED_FIFO
        } else {
            SCHED_OTHER
        }
    } else {
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

pub fn sys_linux_sched_setscheduler(pid: usize, policy: usize, param_ptr: UserPtr<u32>) -> usize {
    let target = if pid == 0 {
        unsafe { crate::kernel::cpu_local::CpuLocal::get().current_task_id() }
    } else {
        crate::interfaces::task::TaskId(pid)
    };

    if !is_supported_policy(policy) {
        return linux_inval();
    }

    let prio = if !param_ptr.is_null() {
        match param_ptr.read() {
            Ok(v) => v,
            Err(e) => return e,
        }
    } else {
        0
    };

    let Some((min_prio, max_prio)) = policy_priority_bounds(policy) else {
        return linux_inval();
    };
    if prio < min_prio || prio > max_prio {
        return linux_inval();
    }

    if let Some(task_arc) = crate::kernel::task::get_task(target) {
        let mut task = task_arc.lock();
        task.priority = if policy == SCHED_OTHER {
            DEFAULT_NORMAL_PRIORITY
        } else {
            linux_rt_to_internal_priority(prio)
        };
        task.rt_group_id = if policy == SCHED_FIFO || policy == SCHED_RR {
            1
        } else {
            0
        };
        0
    } else {
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

pub fn sys_linux_sched_getparam(pid: usize, param_ptr: UserPtr<u32>) -> usize {
    if param_ptr.is_null() {
        return linux_fault();
    }
    let target = if pid == 0 {
        unsafe { crate::kernel::cpu_local::CpuLocal::get().current_task_id() }
    } else {
        crate::interfaces::task::TaskId(pid)
    };

    if let Some(task_arc) = crate::kernel::task::get_task(target) {
        let task = task_arc.lock();
        let prio = if task.rt_group_id > 0 {
            internal_to_linux_rt_priority(task.priority)
        } else {
            0
        };
        if param_ptr.write(&prio).is_err() {
            return linux_fault();
        }
        0
    } else {
        linux_errno(crate::modules::posix_consts::errno::ESRCH)
    }
}

pub fn sys_linux_sched_setparam(pid: usize, param_ptr: UserPtr<u32>) -> usize {
    sys_linux_sched_setscheduler(pid, 0, param_ptr)
}

pub fn sys_linux_sched_get_priority_max(policy: usize) -> usize {
    match policy {
        SCHED_FIFO | SCHED_RR => 99,
        SCHED_OTHER => 0,
        _ => linux_inval(),
    }
}

pub fn sys_linux_sched_get_priority_min(policy: usize) -> usize {
    match policy {
        SCHED_FIFO | SCHED_RR => 1,
        SCHED_OTHER => 0,
        _ => linux_inval(),
    }
}

pub fn sys_linux_sched_rr_get_interval(pid: usize, tp_ptr: UserPtr<u8>) -> usize {
    if tp_ptr.is_null() {
        return linux_fault();
    }

    let target = match resolve_target_task(pid) {
        Ok(tid) => tid,
        Err(errno) => return errno,
    };
    if crate::kernel::task::get_task(target).is_none() {
        return linux_errno(crate::modules::posix_consts::errno::ESRCH);
    }

    let slice_ns = crate::config::KernelConfig::time_slice() as i64;
    let interval = crate::modules::linux_compat::types::LinuxTimespec {
        tv_sec: slice_ns / 1_000_000_000,
        tv_nsec: slice_ns % 1_000_000_000,
    };

    if with_user_write_bytes(
        tp_ptr.addr,
        core::mem::size_of::<crate::modules::linux_compat::types::LinuxTimespec>(),
        |dst| {
            if dst.len()
                < core::mem::size_of::<crate::modules::linux_compat::types::LinuxTimespec>()
            {
                return linux_fault();
            }
            dst[..8].copy_from_slice(&(interval.tv_sec as u64).to_ne_bytes());
            dst[8..16].copy_from_slice(&(interval.tv_nsec as u64).to_ne_bytes());
            0
        },
    )
    .is_err()
    {
        return linux_fault();
    }
    0
}

pub fn sys_linux_sched_yield() -> usize {
    crate::kernel::syscalls::sys_yield()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn policy_bounds_match_linux_contract() {
        assert_eq!(policy_priority_bounds(SCHED_OTHER), Some((0, 0)));
        assert_eq!(policy_priority_bounds(SCHED_FIFO), Some((1, 99)));
        assert_eq!(policy_priority_bounds(SCHED_RR), Some((1, 99)));
        assert_eq!(policy_priority_bounds(999), None);
    }

    #[test_case]
    fn rt_priority_mapping_preserves_order() {
        let high = linux_rt_to_internal_priority(99);
        let mid = linux_rt_to_internal_priority(50);
        let low = linux_rt_to_internal_priority(1);

        assert!(high < mid && mid < low);
        assert!(internal_to_linux_rt_priority(high) >= internal_to_linux_rt_priority(mid));
        assert!(internal_to_linux_rt_priority(mid) >= internal_to_linux_rt_priority(low));
    }
}
