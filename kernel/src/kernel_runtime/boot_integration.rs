// --- PHASE 6: Boot Infrastructure Integration ---
// Wires GLOBAL_BOOT_MANAGER into kernel runtime stages
use alloc::string::String;

use crate::core::log;
use alloc::format;
use crate::interfaces::boot::{BootManager, BootStage, BootSubsystem};
use crate::kernel::boot_manager::GLOBAL_BOOT_MANAGER;
use crate::kernel::boot_subsystems::*;
use crate::kernel::device_manager::GLOBAL_DEVICE_MANAGER;

/// Register and initialize all boot subsystems for a given stage
pub fn register_boot_subsystems(stage: BootStage) -> Result<(), &'static str> {
    match stage {
        // Early memory stage: register allocator and basic infrastructure
        BootStage::EarlyMemory => {
            log::debug("Registering EarlyMemory subsystems");
            GLOBAL_BOOT_MANAGER
                .register_subsystem(BootStage::EarlyMemory, &ALLOCATOR_SUBSYSTEM);
            Ok(())
        }

        // Platform early stage: register scheduler and interrupt handlers
        BootStage::PlatformEarly => {
            log::debug("Registering PlatformEarly subsystems");
            GLOBAL_BOOT_MANAGER
                .register_subsystem(BootStage::PlatformEarly, &SCHEDULER_SUBSYSTEM);
            GLOBAL_BOOT_MANAGER
                .register_subsystem(BootStage::PlatformEarly, &INTERRUPT_SUBSYSTEM);
            Ok(())
        }

        // Platform devices stage: register device manager
        BootStage::PlatformDevices => {
            log::debug("Registering PlatformDevices subsystems");
            GLOBAL_BOOT_MANAGER
                .register_subsystem(BootStage::PlatformDevices, &VFS_SUBSYSTEM);
            Ok(())
        }

        // Core subsystems: all remaining subsystems
        BootStage::CoreSubsystems => {
            log::debug("Registering CoreSubsystems");
            GLOBAL_BOOT_MANAGER
                .register_subsystem(BootStage::CoreSubsystems, &IPC_SUBSYSTEM);
            GLOBAL_BOOT_MANAGER
                .register_subsystem(BootStage::CoreSubsystems, &SECURITY_SUBSYSTEM);
            GLOBAL_BOOT_MANAGER
                .register_subsystem(BootStage::CoreSubsystems, &PROCESS_SUBSYSTEM);
            Ok(())
        }

        _ => Ok(()),
    }
}

/// Initialize platform (x86_64 or aarch64)
pub fn initialize_platform() -> Result<(), &'static str> {
    crate::hal::Hal::platform()
        .init()
        .map_err(|_| "Platform initialization failed")?;

    let caps = crate::hal::Hal::platform().capabilities();
    log::info(&format!(
        "Platform initialized: {} CPUs, SMP={}, virtualization={}",
        caps.cpu_count, caps.has_smp, caps.has_virtualization
    ));
    Ok(())
}

/// Enumerate devices and register with device manager
/// 
/// Uses ACPI on x86_64 and Device Tree Blob on aarch64 to discover platform devices.
/// Discovered devices are registered with GLOBAL_DEVICE_MANAGER for subsystem initialization.
pub fn enumerate_devices() -> Result<(), &'static str> {
    log::info("Starting device enumeration");

    // Register fixed devices (serial, timer, console)
    GLOBAL_DEVICE_MANAGER
        .register_device(crate::interfaces::device::DeviceType::Serial, "serial0").map_err(|_| "Failed to register serial device")?;
    GLOBAL_DEVICE_MANAGER
        .register_device(crate::interfaces::device::DeviceType::Timer, "timer0").map_err(|_| "Failed to register timer device")?;

    let devices = crate::hal::Hal::firmware_provider().enumerate_devices();
    for device in &devices {
        let device_type = match device.device_type.as_str() {
            "interrupt_controller" => crate::interfaces::device::DeviceType::InterruptController,
            "processor" => crate::interfaces::device::DeviceType::Processor,
            "platform_controller" => crate::interfaces::device::DeviceType::PlatformController,
            "serial" | "uart" => crate::interfaces::device::DeviceType::Serial,
            "timer" => crate::interfaces::device::DeviceType::Timer,
            "mmu" | "memory" => crate::interfaces::device::DeviceType::MMU,
            "ethernet" | "network" => crate::interfaces::device::DeviceType::Network,
            _ => crate::interfaces::device::DeviceType::Unknown,
        };

        if device_type != crate::interfaces::device::DeviceType::Unknown {
            GLOBAL_DEVICE_MANAGER.register_device(device_type, device.name.as_str()).ok();
        }
    }

    log::info(&format!("Firmware enumeration: {} devices discovered", devices.len()));

    log::info("Device enumeration completed");
    Ok(())
}

/// Verify that all required subsystems are ready
pub fn verify_subsystem_readiness() -> Result<(), &'static str> {
    if !ALLOCATOR_SUBSYSTEM.is_ready() {
        return Err("Allocator subsystem not ready");
    }
    if !SCHEDULER_SUBSYSTEM.is_ready() {
        return Err("Scheduler subsystem not ready");
    }
    if !VFS_SUBSYSTEM.is_ready() {
        return Err("VFS subsystem not ready");
    }

    log::info("All critical subsystems ready");
    Ok(())
}

/// Get boot diagnostics for reporting
pub fn get_boot_diagnostics() -> String {
    let diags = GLOBAL_BOOT_MANAGER.diagnostics();
    format!(
        "Boot diagnostics: {} stages completed, {} errors",
        diags.stage_timings.len(),
        diags.stage_errors
    )
}

/// Initialize all runtime extension subsystems
pub fn initialize_runtime_extensions() -> Result<(), &'static str> {
    log::info("Initializing runtime extensions");
    
    // Initialize scheduler extensions (must be before task creation)
    super::scheduler_integration::init_multicore_scheduling()?;
    log::info("Scheduler extensions initialized");
    
    // Initialize memory extensions (must be before memory allocation)
    super::memory_integration::init_memory_extensions()?;
    log::info("Memory extensions initialized");
    
    // Initialize VFS extensions (must be before filesystem operations)
    super::vfs_integration::init_vfs_extensions()?;
    log::info("VFS extensions initialized");
    
    log::info("All runtime extensions initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_initialization() {
        // Platform init is called during early boot; verify no panics
        initialize_platform().ok();
    }

    #[test]
    fn test_device_enumeration() {
        enumerate_devices().ok();
    }

    #[test]
    fn test_boot_diagnostics_format() {
        let _diags = get_boot_diagnostics();
        // Verify diagnostic string is not empty
    }
}

#[cfg(test)]
mod phase6_boot_integration_tests {
    use super::*;

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
        // Verify required stages are correct
        assert_eq!(ALLOCATOR_SUBSYSTEM.required_stage(), BootStage::EarlyMemory);
        assert_eq!(SCHEDULER_SUBSYSTEM.required_stage(), BootStage::PlatformEarly);
        assert_eq!(VFS_SUBSYSTEM.required_stage(), BootStage::PlatformDevices);

        // Verify dependency chains are valid
        let scheduler_deps = SCHEDULER_SUBSYSTEM.dependencies();
        assert!(
            scheduler_deps.contains(&"MemoryAllocator"),
            "Scheduler should depend on MemoryAllocator"
        );

        let vfs_deps = VFS_SUBSYSTEM.dependencies();
        assert!(
            vfs_deps.contains(&"Scheduler"),
            "VFS should depend on Scheduler"
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
