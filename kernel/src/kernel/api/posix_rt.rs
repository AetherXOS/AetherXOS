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

/// Gets the scheduling policy and parameters for the specified thread
/// Emulates `pthread_getschedparam`
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_getschedparam(thread_id: u64, policy: *mut SchedPolicy, param: *mut SchedParam) -> c_int {
    if policy.is_null() || param.is_null() {
        return crate::modules::posix_consts::errno::EINVAL;
    }

    let tid = task::TaskId(thread_id as usize);
    if let Some(task_arc) = task::get_task(tid) {
        let t = task_arc.lock();

        let task_policy = if t.rt_budget_ns == u64::MAX {
            SchedPolicy::Fifo
        } else if t.rt_budget_ns == crate::config::KernelConfig::mlfq_base_slice_ns() {
            SchedPolicy::Rr
        } else if t.priority == crate::kernel::task::IDLE_PRIORITY {
            SchedPolicy::Other
        } else {
            SchedPolicy::Other // Default/fallback
        };

        unsafe {
            *policy = task_policy;
            (*param).sched_priority = if t.priority == crate::kernel::task::IDLE_PRIORITY {
                0
            } else {
                crate::kernel::task::IDLE_PRIORITY as i32 - t.priority as i32
            };
        }

        0 // Success
    } else {
        crate::modules::posix_consts::errno::ESRCH
    }
}

/// Sets the scheduling policy and parameters for the specified thread
/// Emulates `pthread_setschedparam`
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_setschedparam(thread_id: u64, policy: SchedPolicy, param: *const SchedParam) -> c_int {
    if param.is_null() {
        return crate::modules::posix_consts::errno::EINVAL;
    }
    
    let p = unsafe { &*param };
    
    // Validate POSIX priority (1 to 99 for RT)
    if policy == SchedPolicy::Fifo || policy == SchedPolicy::Rr {
        if p.sched_priority < 1 || p.sched_priority > 99 {
            return crate::modules::posix_consts::errno::EINVAL;
        }
    }
    
    let native_priority = if policy == SchedPolicy::Other || policy == SchedPolicy::Batch || policy == SchedPolicy::Idle {
        crate::kernel::task::IDLE_PRIORITY // Lowest native priority
    } else {
        // Map POSIX 1-99 to native priority namespace
        (crate::kernel::task::IDLE_PRIORITY as i32 - p.sched_priority) as u8
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
                t.rt_budget_ns = crate::config::KernelConfig::mlfq_base_slice_ns();
                t.rt_period_ns = crate::config::KernelConfig::mlfq_base_slice_ns();
            },
            _ => {}
        }
        
        0 // Success
    } else {
        crate::modules::posix_consts::errno::ESRCH
    }
}

#[repr(C)]
pub struct pthread_mutex_t {
    inner: *mut PiMutex<u8>,
}

/// Destroys a PI-capable mutex.
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_mutex_destroy(mutex: *mut pthread_mutex_t) -> c_int {
    if mutex.is_null() || unsafe { (*mutex).inner.is_null() } {
        return crate::modules::posix_consts::errno::EINVAL;
    }

    #[cfg(feature = "rtos_strict")]
    {
        // For simplicity in static pools, we just nullify it.
        // A true implementation might return it to MUTEX_POOL.
        unsafe { (*mutex).inner = core::ptr::null_mut(); }
        return 0;
    }

    #[cfg(not(feature = "rtos_strict"))]
    {
        return crate::modules::posix_consts::errno::ENOSYS;
    }
}

/// Initializes a PI-capable mutex from the static RT pool.
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_mutex_init(mutex: *mut pthread_mutex_t, _attr: *const c_void) -> c_int {
    if mutex.is_null() {
        return crate::modules::posix_consts::errno::EINVAL;
    }

    #[cfg(feature = "rtos_strict")]
    {
        if let Some(pi_mutex_ref) = MUTEX_POOL.alloc() {
            unsafe {
                (*mutex).inner = pi_mutex_ref as *mut _;
            }
            return 0;
        }
        return crate::modules::posix_consts::errno::ENOMEM;
    }

    #[cfg(not(feature = "rtos_strict"))]
    {
        // Fallback or simple allocation if not in strict mode
        return crate::modules::posix_consts::errno::ENOSYS;
    }
}

/// Tries to lock a PI mutex without blocking.
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_mutex_trylock(mutex: *mut pthread_mutex_t) -> c_int {
    if mutex.is_null() || unsafe { (*mutex).inner.is_null() } {
        return crate::modules::posix_consts::errno::EINVAL;
    }

    let (tid, prio) = get_current_task_info();
    let pi_mutex = unsafe { &*(*mutex).inner };

    // Attempt to lock without blocking
    if let Some(guard) = pi_mutex.try_lock(tid, prio) {
        core::mem::forget(guard);
        0
    } else {
        crate::modules::posix_consts::errno::EBUSY
    }
}

/// Locks a PI mutex, boosting the owner's priority if contention occurs.
#[cfg(feature = "rtos_posix")]
#[unsafe(no_mangle)]
pub extern "C" fn pthread_mutex_lock(mutex: *mut pthread_mutex_t) -> c_int {
    if mutex.is_null() || unsafe { (*mutex).inner.is_null() } {
        return crate::modules::posix_consts::errno::EINVAL;
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
        return crate::modules::posix_consts::errno::EINVAL;
    }

    let tid = unsafe { CpuLocal::get() }.current_task_id();
    let pi_mutex = unsafe { &*(*mutex).inner };

    // Check if we actually own it (Strict RTOS check)
    if pi_mutex.owner() != Some(tid) {
        return crate::modules::posix_consts::errno::EPERM;
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
        crate::kernel::task::IDLE_PRIORITY
    };
    (tid, prio)
}
