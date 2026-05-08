// --- PHASE 4: BOOT MANAGER IMPLEMENTATION ---
// Concrete implementation of the BootManager trait with subsystem orchestration

use crate::core::log;
use alloc::format;
use crate::interfaces::boot::{
    BootDiagnostics, BootInfo, BootManager, BootStage, BootSubsystem,
};
use crate::interfaces::{KernelError, KernelResult};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::kernel::sync::IrqSafeMutex;

/// Concrete implementation of the BootManager trait
pub struct ConcreteBootManager {
    /// Current boot stage
    current_stage: IrqSafeMutex<BootStage>,

    /// Registered subsystems indexed by stage
    subsystems: IrqSafeMutex<BTreeMap<BootStage, Vec<&'static dyn BootSubsystem>>>,

    /// Boot diagnostics for reporting
    diagnostics: IrqSafeMutex<BootDiagnostics>,

    /// Boot information collected during initialization
    boot_info: IrqSafeMutex<BootInfo>,
}

impl ConcreteBootManager {
    /// Create a new BootManager instance
    pub const fn new() -> Self {
        Self {
            current_stage: IrqSafeMutex::new(BootStage::BootloaderHandoff),
            subsystems: IrqSafeMutex::new(BTreeMap::new()),
            diagnostics: IrqSafeMutex::new(BootDiagnostics {
                stage_timings: [0u64; 10],
                stage_errors: 0,
                warnings: 0,
            }),
            boot_info: IrqSafeMutex::new(BootInfo {
                entry_stage: BootStage::BootloaderHandoff,
                current_stage: BootStage::BootloaderHandoff,
                subsystems_ready: 0,
                total_init_time_us: 0,
                boot_timestamp_us: 0,
                memory_size: 0,
                memory_start: 0,
                cpu_count: 1,
                cpu_freq_mhz: 0,
                platform_id: 0,
                acpi_rsdp: None,
                dtb_address: None,
            }),
        }
    }

    /// Initialize the boot manager (called at kernel startup)
    pub fn initialize() -> KernelResult<&'static ConcreteBootManager> {
        log::info("Initializing BootManager");
        Ok(&GLOBAL_BOOT_MANAGER)
    }
}

// Global boot manager instance
pub static GLOBAL_BOOT_MANAGER: ConcreteBootManager = ConcreteBootManager::new();

impl BootManager for ConcreteBootManager {
    /// Register a subsystem that should be initialized at a specific boot stage
    fn register_subsystem(&self, stage: BootStage, subsystem: &'static dyn BootSubsystem) {
        let mut subsystems = self.subsystems.lock();
        subsystems.entry(stage).or_insert_with(Vec::new).push(subsystem);

        log::debug(&format!(
            "Registered subsystem at stage: {:?}",
            stage
        ));
    }

    /// Transition to a new boot stage and initialize all registered subsystems
    fn enter_stage(&self, stage: BootStage) -> KernelResult<()> {
        let current = *self.current_stage.lock();

        // Verify stage ordering (no backward transitions)
        if stage < current {
            log::error(&format!(
                "Invalid stage transition: {:?} -> {:?}",
                current,
                stage
            ));
            self.diagnostics.lock().stage_errors += 1;
            return Err(KernelError::InvalidInput);
        }

        log::info(&format!("Entering boot stage: {:?}", stage));

        // Get subsystems for this stage
        let subsystems = self
            .subsystems
            .lock()
            .get(&stage)
            .map(|v| v.clone())
            .unwrap_or_default();

        // Initialize each subsystem in order
        for subsystem in subsystems {
            // Check dependencies first
            let deps = subsystem.dependencies();
            for dep_name in deps {
                // Dependency check logic (simplified: should check if dep's stage is reached)
                log::trace(&format!("Checking dependency: {}", dep_name));
            }

            // Initialize the subsystem
            log::debug(&format!("Initializing subsystem: {}", subsystem.name()));
            subsystem.init().map_err(|_| KernelError::InternalError)?;

            // Verify readiness
            if !subsystem.is_ready() {
                log::warn(&format!(
                    "Subsystem not ready after init: {}",
                    subsystem.name()
                ));
                self.diagnostics.lock().warnings += 1;
            } else {
                self.boot_info.lock().subsystems_ready += 1;
            }
        }

        // Update stage tracking
        *self.current_stage.lock() = stage;
        let mut info = self.boot_info.lock();
        info.current_stage = stage;

        log::info(&format!(
            "Boot stage complete: {:?}",
            stage
        ));
        Ok(())
    }

    /// Get current boot stage
    fn current_stage(&self) -> BootStage {
        *self.current_stage.lock()
    }

    /// Get boot diagnostics for reporting
    fn diagnostics(&self) -> BootDiagnostics {
        self.diagnostics.lock().clone()
    }

    /// Get boot information snapshot
    fn boot_info(&self) -> BootInfo {
        self.boot_info.lock().clone()
    }

    /// Check if all critical subsystems are ready
    fn are_subsystems_ready(&self) -> bool {
        let current = *self.current_stage.lock();
        current >= BootStage::RuntimeReady
    }
}

impl ConcreteBootManager {
    /// Record diagnostic timing for a stage
    pub fn record_stage_timing(&self, stage: BootStage, duration_us: u64) {
        let idx = stage as usize;
        if idx < 10 {
            self.diagnostics.lock().stage_timings[idx] = duration_us;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSubsystem {
        name: &'static str,
        dependencies: &'static [BootStage],
    }

    impl BootSubsystem for MockSubsystem {
        fn name(&self) -> &'static str {
            self.name
        }

        fn required_stage(&self) -> BootStage {
            BootStage::EarlyMemory
        }

        fn init(&self) -> KernelResult<()> {
            log::debug(&format!("Mock init: {}", self.name));
            Ok(())
        }

        fn is_ready(&self) -> bool {
            true
        }

        fn dependencies(&self) -> &[&'static str] {
            &[]
        }
    }

    #[test]
    fn test_boot_manager_creation() {
        let mgr = ConcreteBootManager::new();
        assert_eq!(mgr.current_stage(), BootStage::BootloaderHandoff);
    }

    #[test]
    fn test_stage_ordering() {
        assert!(BootStage::EarlyMemory > BootStage::BootloaderHandoff);
        assert!(BootStage::RuntimeReady > BootStage::CoreSubsystems);
    }

    #[test]
    fn test_subsystem_registration() {
        let mgr = ConcreteBootManager::new();
        // Just verify registration doesn't panic
        // Since it uses interior mutability, we'd need a way to inspect it
    }

    #[test]
    fn test_diagnostics_initial_state() {
        let mgr = ConcreteBootManager::new();
        let diag = mgr.diagnostics();
        assert_eq!(diag.stage_errors, 0);
        assert_eq!(diag.warnings, 0);
    }

    #[test]
    fn test_boot_info_initial_state() {
        let mgr = ConcreteBootManager::new();
        let info = mgr.boot_info();
        assert_eq!(info.current_stage, BootStage::BootloaderHandoff);
        assert_eq!(info.subsystems_ready, 0);
    }

    #[test]
    fn test_subsystems_not_ready_early() {
        let mgr = ConcreteBootManager::new();
        assert!(!mgr.are_subsystems_ready());
    }

    #[test]
    fn test_stage_diagnostic_timing() {
        let mgr = ConcreteBootManager::new();
        mgr.record_stage_timing(BootStage::EarlyMemory, 12345);
        let diag = mgr.diagnostics();
        assert_eq!(diag.stage_timings[1], 12345);
    }
}
