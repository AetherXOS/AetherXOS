use super::super::*;

/// `brk(2)` — legacy process heap end management.
pub fn sys_linux_brk(addr: usize) -> usize {
    #[cfg(feature = "process_abstraction")]
    {
        if let Some(pid) = current_process_id() {
            if let Some(process) = crate::kernel::launch::process_arc_by_id(crate::interfaces::task::ProcessId(pid)) {
                match process.set_brk(addr as u64) {
                    Ok(new_brk) => return new_brk as usize,
                    Err(_) => {
                        // Return current break on error as per linux spec
                        return process.heap_break.load(core::sync::atomic::Ordering::Relaxed) as usize;
                    }
                }
            }
        }
    }
    
    // Fallback if process abstraction is disabled (unlikely in production)
    if addr == 0 {
        return linux::BRK_START as usize;
    }
    addr
}
