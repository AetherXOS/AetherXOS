//! x86_64 Specific Task Architecture Logic for AetherXOS.

pub struct ArchTask;

impl ArchTask {
    #[cfg(target_arch = "x86_64")]
    pub fn initial_stack_frame_image(entry: u64) -> [u64; 8] {
        [
            0u64, 0u64, 0u64, 0u64, 0u64, 0u64,
            entry,
            Self::initial_task_return_trap as *const () as usize as u64,
        ]
    }

    #[cfg(target_arch = "x86_64")]
    extern "C" fn initial_task_return_trap() -> ! {
        panic!("kernel task entry returned unexpectedly")
    }
}
