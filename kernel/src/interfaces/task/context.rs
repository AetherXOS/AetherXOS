// Re-export the HAL CpuContext as the task `Context` to provide a stable,
// architecture-agnostic API for task context operations.
//
// This removes localized `cfg(target_arch)` blocks from higher-level code
// and centralizes architecture-specific representations inside `hal/`.

pub use crate::hal::abstractions::CpuContext as Context;

impl Context {
    /// Alias for the HAL method: return the instruction pointer (IP/RIP/ELR)
    #[inline(always)]
    pub fn rip(&self) -> u64 {
        self.instruction_pointer()
    }

    /// Return stack pointer
    #[inline(always)]
    pub fn sp(&self) -> u64 {
        self.stack_pointer()
    }

    /// Return frame pointer
    #[inline(always)]
    pub fn fp(&self) -> u64 {
        self.frame_pointer()
    }
}
