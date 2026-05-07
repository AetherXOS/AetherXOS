use crate::interfaces::task::{TaskId, TaskState};
use crate::interfaces::memory::PageAllocator;
use crate::interfaces::cpu::CpuRegisters;
use alloc::boxed::Box;

const KTHREAD_STACK_PAGES: usize = 4;
const KTHREAD_STACK_SIZE: usize = KTHREAD_STACK_PAGES * 4096;

/// Trampoline that runs a boxed closure stored on the kernel stack.
/// The scheduler jumps here; `rdi` / `x0` carries the raw pointer to the
/// heap-allocated `Box<dyn FnOnce()>`.
extern "C" fn kthread_trampoline(data: usize) -> ! {
    let closure: Box<Box<dyn FnOnce()>> =
        unsafe { Box::from_raw(data as *mut Box<dyn FnOnce()>) };
    closure();
    // After the closure returns, terminate this kernel thread.
    crate::kernel::rt_preemption::request_forced_reschedule();
    loop {
        crate::hal::HAL::cpu_relax();
    }
}

/// Spawn a new Kernel Thread.
/// Kernel threads run in Ring 0 and share the kernel address space.
pub fn spawn_kthread<F>(name: &str, f: F) -> TaskId
where
    F: FnOnce() + Send + 'static,
{
    let tid = crate::kernel::task::alloc_tid();

    // Allocate a kernel stack via the page allocator.
    let kernel_stack_top = {
        let mut alloc = crate::modules::allocators::selector::ActivePageAllocator::new();
        let base = alloc
            .allocate_pages(KTHREAD_STACK_PAGES.trailing_zeros() as u8)
            .expect("[KTHREAD] OOM: cannot allocate kernel stack");
        (base + KTHREAD_STACK_SIZE) as u64
    };

    // Read the current page table root (kernel threads share the kernel address space).
    let cr3 = crate::hal::cpu::ArchCpuRegisters::read_page_table_root();

    // Box the closure and convert to a raw pointer for the trampoline.
    let boxed: Box<Box<dyn FnOnce()>> = Box::new(Box::new(f));
    let data_ptr = Box::into_raw(boxed) as u64;

    let entry = kthread_trampoline as *const () as u64;

    let task_arc = crate::interfaces::task::KernelTask::new_shared(
        tid,
        10, // Default priority
        0,  // No deadline
        0,  // No burst time
        kernel_stack_top,
        cr3,
        entry,
    );

    {
        let mut t = task_arc.lock();
        t.name = alloc::string::String::from(name);
        t.state = TaskState::Ready;
        // Store the closure pointer in rdi (x86_64) or x0 (aarch64) so the
        // trampoline receives it as its first argument.
        #[cfg(target_arch = "x86_64")]
        {
            t.context.rdi = data_ptr;
        }
        #[cfg(target_arch = "aarch64")]
        {
            t.context.x[0] = data_ptr;
        }
    }

    crate::klog_info!("[KTHREAD] Spawned kernel thread '{}'", name);
    crate::kernel::task::spawn_task(task_arc)
}

/// Kernel Worker Thread for background tasks.
pub fn kernel_worker_loop() {
    loop {
        // Process workqueue entries
        crate::hal::HAL::cpu_relax();
    }
}
