//! POSIX Real-Time Extensions Facade (VxWorks/Linux-RT Compatibility)
//! 
//! This module provides a POSIX-compliant API for scheduling and 
//! Real-Time thread management, mapping directly onto the native
//! AetherXOS hard real-time scheduler.

use crate::kernel::task;
use crate::kernel::cpu_local::CpuLocal;
use crate::kernel::memory::rt_pools::MUTEX_POOL;
use crate::kernel::pi_mutex::PiMutex;
use core::ffi::{c_int, c_void};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedPolicy {
    Other = 0,
    Fifo = 1,
    Rr = 2,
    Batch = 3,
    Idle = 5,
    Deadline = 6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SchedParam {
    pub sched_priority: i32,
}

/// Sets the scheduling policy and parameters for the specified thread
/// Emulates `pthread_setschedparam`
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_setschedparam(thread_id: u64, policy: SchedPolicy, param: *const SchedParam) -> c_int {
    if param.is_null() {
        return 22; // EINVAL
    }
    
    let p = unsafe { &*param };
    
    // Validate POSIX priority (1 to 99 for RT)
    if policy == SchedPolicy::Fifo || policy == SchedPolicy::Rr {
        if p.sched_priority < 1 || p.sched_priority > 99 {
            return 22; // EINVAL
        }
    }
    
    let native_priority = if policy == SchedPolicy::Other || policy == SchedPolicy::Batch || policy == SchedPolicy::Idle {
        255 // Lowest native priority
    } else {
        // Map POSIX 1-99 to native priority namespace
        (255 - p.sched_priority) as u8
    };

    let tid = task::TaskId(thread_id as usize);
    if let Some(task_arc) = task::get_task(tid) {
        let mut t = task_arc.lock();
        
        t.priority = native_priority;
        
        match policy {
            SchedPolicy::Fifo => {
                t.rt_budget_ns = u64::MAX; 
                t.rt_period_ns = u64::MAX;
            },
            SchedPolicy::Rr => {
                t.rt_budget_ns = 4_000_000; // 4ms
                t.rt_period_ns = 4_000_000;
            },
            _ => {}
        }
        
        0 // Success
    } else {
        3 // ESRCH
    }
}

#[repr(C)]
pub struct pthread_mutex_t {
    inner: *mut PiMutex<u8>,
}

/// Initializes a PI-capable mutex from the static RT pool.
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_mutex_init(mutex: *mut pthread_mutex_t, _attr: *const c_void) -> c_int {
    if mutex.is_null() {
        return 22; // EINVAL
    }

    #[cfg(feature = "rtos_strict")]
    {
        if let Some(pi_mutex_ref) = MUTEX_POOL.alloc() {
            unsafe {
                (*mutex).inner = pi_mutex_ref as *mut _;
            }
            return 0;
        }
        return 12; // ENOMEM (Pool exhausted)
    }

    #[cfg(not(feature = "rtos_strict"))]
    {
        // Fallback or simple allocation if not in strict mode
        return 38; // ENOSYS
    }
}

/// Locks a PI mutex, boosting the owner's priority if contention occurs.
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_mutex_lock(mutex: *mut pthread_mutex_t) -> c_int {
    if mutex.is_null() || unsafe { (*mutex).inner.is_null() } {
        return 22; // EINVAL
    }

    let (tid, prio) = get_current_task_info();
    let pi_mutex = unsafe { &*(*mutex).inner };
    
    // Perform the PI-aware lock
    let guard = pi_mutex.lock(tid, prio);
    
    // We "forget" the guard because POSIX unlock is a separate call.
    // The lock state is maintained in the PiMutex structure itself.
    core::mem::forget(guard);
    
    0
}

/// Unlocks a PI mutex and restores the owner's original priority.
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_mutex_unlock(mutex: *mut pthread_mutex_t) -> c_int {
    if mutex.is_null() || unsafe { (*mutex).inner.is_null() } {
        return 22; // EINVAL
    }

    let tid = unsafe { CpuLocal::get() }.current_task_id();
    let pi_mutex = unsafe { &*(*mutex).inner };

    // Check if we actually own it (Strict RTOS check)
    if pi_mutex.owner() != Some(tid) {
        return 1; // EPERM
    }

    // Safety: pthread_mutex_lock core::mem::forgot the guard,
    // so we manually call the internal unlock logic here.
    unsafe {
        pi_mutex.unlock_from_posix(tid);
    }

    0
}

fn get_current_task_info() -> (task::TaskId, u8) {
    let cpu = unsafe { CpuLocal::get() };
    let tid = cpu.current_task_id();
    let prio = if let Some(task_arc) = task::get_task(tid) {
        task_arc.lock().priority
    } else {
        255
    };
    (tid, prio)
}
