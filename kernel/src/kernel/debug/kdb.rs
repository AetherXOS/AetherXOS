use crate::interfaces::task::TaskId;
use crate::kernel::task::get_task;

/// Aether-KDB: The Kernel Post-Mortem Debugger.
/// Triggered on panics or manual breaks.
pub struct Kdb;

impl Kdb {
    /// Enter the interactive debugger shell.
    pub fn enter() {
        crate::klog_err!("**************************************************");
        crate::klog_err!("* WELCOME TO AETHER-KDB (KERNEL DEBUGGER)        *");
        crate::klog_err!("**************************************************");
        Self::print_summary();
    }

    fn print_summary() {
        let tid = crate::hal::HAL::current_task();
        crate::klog_err!("Current Task ID: {:?}", tid);
        if let Some(task) = get_task(TaskId(tid)) {
            let t = task.lock();
            crate::klog_err!("Task Name: {}", t.name);
            crate::klog_err!("Task State: {:?}", t.state);
        }
    }

    /// Dump memory at a specific address.
    pub fn dump_mem(addr: u64, size: usize) {
        let ptr = addr as *const u8;
        crate::klog_err!("Memory Dump at {:#x}:", addr);
        for i in 0..size {
            unsafe {
                crate::klog_err!("{:02x} ", *ptr.add(i));
            }
        }
    }
}
