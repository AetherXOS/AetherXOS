//! Boot Graph Orchestrator - Automated dependency resolution for kernel boot.
//!
//! This module provides a high-level orchestrator that can reach specific boot stages
//! by automatically registering and initializing required subsystems.

use crate::interfaces::boot::{BootManager, BootStage};
use crate::kernel::boot_manager::GLOBAL_BOOT_MANAGER;
use crate::kernel::boot_integration;
use crate::interfaces::KernelResult;
use crate::core::log;

pub struct BootOrchestrator {
    manager: &'static dyn BootManager,
}

impl BootOrchestrator {
    pub const fn new() -> Self {
        Self {
            manager: &GLOBAL_BOOT_MANAGER,
        }
    }

    /// Reach a target boot stage, automatically handling all intermediate steps.
    pub fn reach(&self, target: BootStage) -> KernelResult<()> {
        let current = self.manager.current_stage();
        
        if target <= current {
            return Ok(());
        }

        log::info(&alloc::format!("Orchestrator: Reaching target stage {:?}", target));

        // Iterate through all stages up to the target
        for stage_val in (current as u8 + 1)..=(target as u8) {
            let stage = unsafe { core::mem::transmute::<u8, BootStage>(stage_val) };
            
            // 1. Auto-register subsystems for this stage
            boot_integration::register_boot_subsystems(stage).map_err(|_| {
                crate::interfaces::KernelError::InternalError
            })?;

            // 2. Enter the stage (this will now use our improved topological sort)
            self.manager.enter_stage(stage)?;
            
            log::debug(&alloc::format!("Orchestrator: Stage {:?} reached successfully", stage));
        }

        Ok(())
    }

    /// Get diagnostics from the underlying manager
    pub fn diagnostics(&self) -> alloc::string::String {
        boot_integration::get_boot_diagnostics()
    }
}

/// Global orchestrator instance
pub static ORCHESTRATOR: BootOrchestrator = BootOrchestrator::new();
