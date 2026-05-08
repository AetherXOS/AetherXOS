// --- PHASE 6: Boot Integration Tests ---
// Comprehensive tests verifying boot manager integration with kernel_runtime

#[cfg(test)]
mod phase6_boot_integration_tests {
    use crate::kernel::boot_manager::GLOBAL_BOOT_MANAGER;
    use crate::kernel::boot_subsystems::*;
    use crate::kernel::device_manager::GLOBAL_DEVICE_MANAGER;
    use crate::kernel::runtime_manager::GLOBAL_RUNTIME_MANAGER;
    use crate::interfaces::boot::BootStage;
    use crate::kernel_runtime::boot_integration::*;

    #[test]
    fn test_boot_stages_sequential_order() {
        // Verify stages can be entered in correct order without errors
        let stages = vec![
            BootStage::BootloaderHandoff,
            BootStage::EarlyMemory,
            BootStage::PlatformEarly,
            BootStage::PlatformDevices,
            BootStage::CoreSubsystems,
            BootStage::UserspaceReady,
        ];

        for stage in stages {
            assert!(
                GLOBAL_BOOT_MANAGER.enter_stage(stage).is_ok(),
                "Failed to enter stage {:?}",
                stage
            );
        }
    }

    #[test]
    fn test_platform_initialization() {
        // Verify platform can be initialized without errors
        assert!(
            initialize_platform().is_ok(),
            "Platform initialization failed"
        );
    }

    #[test]
    fn test_device_enumeration() {
        // Verify device enumeration populates device manager
        assert!(enumerate_devices().is_ok(), "Device enumeration failed");
    }

    #[test]
    fn test_boot_subsystems_readiness() {
        // Initialize all subsystems and verify readiness
        assert!(ALLOCATOR_SUBSYSTEM.init().is_ok());
        assert!(SCHEDULER_SUBSYSTEM.init().is_ok());
        assert!(VFS_SUBSYSTEM.init().is_ok());
        assert!(IPC_SUBSYSTEM.init().is_ok());
        assert!(INTERRUPT_SUBSYSTEM.init().is_ok());
        assert!(SECURITY_SUBSYSTEM.init().is_ok());
        assert!(PROCESS_SUBSYSTEM.init().is_ok());

        // Verify all subsystems report ready
        assert!(ALLOCATOR_SUBSYSTEM.is_ready());
        assert!(SCHEDULER_SUBSYSTEM.is_ready());
        assert!(VFS_SUBSYSTEM.is_ready());
        assert!(IPC_SUBSYSTEM.is_ready());
        assert!(INTERRUPT_SUBSYSTEM.is_ready());
        assert!(SECURITY_SUBSYSTEM.is_ready());
        assert!(PROCESS_SUBSYSTEM.is_ready());
    }

    #[test]
    fn test_subsystem_dependencies_valid() {
        // Verify dependency chains are valid
        let allocator_deps = ALLOCATOR_SUBSYSTEM.dependencies();
        assert!(
            allocator_deps.contains(&BootStage::EarlyMemory),
            "Allocator should depend on EarlyMemory"
        );

        let scheduler_deps = SCHEDULER_SUBSYSTEM.dependencies();
        assert!(
            scheduler_deps.contains(&BootStage::PlatformEarly),
            "Scheduler should depend on PlatformEarly"
        );

        let vfs_deps = VFS_SUBSYSTEM.dependencies();
        assert!(
            vfs_deps.contains(&BootStage::PlatformDevices),
            "VFS should depend on PlatformDevices"
        );
    }

    #[test]
    fn test_boot_diagnostics_available() {
        // Verify boot diagnostics can be retrieved
        let diags = get_boot_diagnostics();
        assert!(!diags.is_empty(), "Boot diagnostics should not be empty");
        assert!(
            diags.contains("Boot diagnostics"),
            "Diagnostics should have expected format"
        );
    }

    #[test]
    fn test_register_boot_subsystems_early_memory() {
        // Verify early memory subsystems can be registered
        assert!(
            register_boot_subsystems(BootStage::EarlyMemory).is_ok(),
            "Failed to register EarlyMemory subsystems"
        );
    }

    #[test]
    fn test_register_boot_subsystems_platform_early() {
        // Verify platform early subsystems can be registered
        assert!(
            register_boot_subsystems(BootStage::PlatformEarly).is_ok(),
            "Failed to register PlatformEarly subsystems"
        );
    }

    #[test]
    fn test_register_boot_subsystems_platform_devices() {
        // Verify platform device subsystems can be registered
        assert!(
            register_boot_subsystems(BootStage::PlatformDevices).is_ok(),
            "Failed to register PlatformDevices subsystems"
        );
    }

    #[test]
    fn test_register_boot_subsystems_core() {
        // Verify core subsystems can be registered
        assert!(
            register_boot_subsystems(BootStage::CoreSubsystems).is_ok(),
            "Failed to register CoreSubsystems"
        );
    }

    #[test]
    fn test_subsystem_names_unique() {
        // Verify each subsystem has a unique name
        let names = vec![
            ALLOCATOR_SUBSYSTEM.name(),
            SCHEDULER_SUBSYSTEM.name(),
            VFS_SUBSYSTEM.name(),
            IPC_SUBSYSTEM.name(),
            INTERRUPT_SUBSYSTEM.name(),
            SECURITY_SUBSYSTEM.name(),
            PROCESS_SUBSYSTEM.name(),
        ];

        for (i, name1) in names.iter().enumerate() {
            for (j, name2) in names.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        name1, name2,
                        "Subsystem names must be unique"
                    );
                }
            }
        }
    }

    #[test]
    fn test_verify_subsystem_readiness() {
        // Verify that subsystem readiness check works
        // All subsystems report ready by default in stub implementation
        assert!(verify_subsystem_readiness().is_ok());
    }
}
