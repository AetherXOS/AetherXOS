//! POSIX Real-Time Extensions Facade (VxWorks/Linux-RT Compatibility)
//! 
//! This module provides a POSIX-compliant API for scheduling and 
//! Real-Time thread management, mapping directly onto the native
//! AetherXOS hard real-time scheduler.

use crate::kernel::task;
use core::ffi::c_int;

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
