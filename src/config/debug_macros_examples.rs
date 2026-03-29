// Example: Autonomous Observability System Usage
// 
// This file demonstrates the new autonomous emission system for logs, traces, 
// and serial output with autonomous prefix generation and newline handling.
//
// Previous approach (manual, tedious):
//     let line = format!("[EARLY SERIAL] x86_64 ap cpu id ready\n");
//     crate::hal::serial::write_raw(&line);
//
// New approach (autonomous, clean):
//     let line = crate::config::serial_autonomous(Boot, "x86_64 ap cpu id ready");
//     crate::hal::serial::write_raw(&line);
//
// Or via runtime configuration:
//     KernelConfig::set_observability_category_enabled(Boot, Some(true));

use crate::config::{serial_autonomous, serial_autonomous_hex, trace_autonomous};
use crate::config::ObservabilityCategory::*;
use crate::config::KernelConfig;

pub mod examples {
    use super::*;

    /// Example: Boot sequence with autonomous serial emission
    /// Output: "[BOOT] Initializing x86_64 bootloader\n"
    pub fn boot_sequence_example() {
        let msg = serial_autonomous(Boot, "Initializing x86_64 bootloader");
        // Send via serial: msg = "[BOOT] Initializing x86_64 bootloader\n"
    }

    /// Example: Memory allocation with hex value
    /// Output: "[MEMORY] frame_allocated=0x1000000\n"
    pub fn memory_allocation_example(frame_addr: u64) {
        let msg = serial_autonomous_hex(Memory, "frame_allocated", frame_addr);
        // Output includes automatic prefix and newline
    }

    /// Example: Granular category-level control at runtime
    pub fn category_control_example() {
        // Enable Boot category logging at runtime
        KernelConfig::set_observability_category_enabled(Boot, Some(true));

        // Disable Memory category debugging  
        KernelConfig::set_observability_category_enabled(Memory, Some(false));

        // Check if a category is enabled (respects both compile-time and runtime)
        let is_boot_enabled = KernelConfig::is_observability_category_enabled(Boot);
        let is_memory_enabled = KernelConfig::is_observability_category_enabled(Memory);
    }

    /// Example: Task/process tracing with autonomous format
    /// Output: "[TASK] fork pid=42\n"
    pub fn task_trace_example(pid: u32) {
        let msg = alloc::format!("[{}] fork pid={}\n", Task.as_str(), pid);
        // Autonomous prefix: "[TASK]"
        // Automatic newline: "\n"
        // No manual "[EARLY SERIAL]" prefix needed
    }

    /// Example: Scheduler events with granular gating
    pub fn scheduler_event_example() {
        if KernelConfig::is_observability_category_enabled(Scheduler) {
            let msg = serial_autonomous(Scheduler, "load balancing triggered");
            // Only emitted if Scheduler category is enabled
        }
    }

    /// Example: Fault handling with priority output
    pub fn fault_handling_example(fault_type: &str, address: u64) {
        if KernelConfig::is_observability_category_enabled(Fault) {
            let msg = serial_autonomous_hex(Fault, fault_type, address);
            // Autonomous: "[FAULT] {fault_type}=0x{address:x}\n"
        }
    }

    /// Example: I/O tracing without manual newlines
    pub fn io_trace_example(operation: &str) {
        // Old way: "let msg = format!("[EARLY SERIAL] io_op {} done\n", operation);"
        // New way:
        let msg = alloc::format!("[{}] io_op {} done\n", Io.as_str(), operation);
        // Prefix and newline are consistent across codebase
    }

    /// Example: Network events with formatted messages  
    pub fn network_event_example(packet_count: u32) {
        // Using the autonomous helper
        let msg = alloc::format!(
            "[{}] received {} packets\n",
            Network.as_str(),
            packet_count
        );
        // Or just use format_args if you have serial_autonomous_fmt available
    }

    /// Example: Core kernel initialization
    pub fn core_init_example() {
        let msg = serial_autonomous(Core, "kernel initialization complete");
        // Output: "[CORE] kernel initialization complete\n"
        
        // Enable debug tracing for this initialization
        KernelConfig::set_debug_trace_enabled(Some(true));
    }
}

/// Example: Using ObservabilityCategory enum values
pub mod category_reference {
    use super::*;

    pub fn show_all_categories() {
        // All available observability categories
        let categories = [
            (Core, "Core kernel"),
            (Boot, "Boot sequence"),
            (Loader, "Module loader"),
            (Task, "Task management"),
            (Memory, "Memory management"),
            (Scheduler, "Task scheduling"),
            (Fault, "Fault handling"),
            (Driver, "Driver operations"),
            (Io, "I/O subsystem"),
            (Network, "Network subsystem"),
        ];

        for (category, description) in &categories {
            println!(
                "Category: {} - {} (id: {})",
                category.as_str(),
                description,
                category.as_u8()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autonomous_message_generation() {
        let msg = serial_autonomous(Boot, "test message");
        assert_eq!(msg, "[BOOT] test message\n");
    }

    #[test]
    fn test_autonomous_hex_generation() {
        let msg = serial_autonomous_hex(Memory, "addr", 0xDEADBEEF);
        assert_eq!(msg, "[MEMORY] addr=0xdeadbeef\n");
    }

    #[test]
    fn test_category_strings() {
        assert_eq!(Core.as_str(), "CORE");
        assert_eq!(Boot.as_str(), "BOOT");
        assert_eq!(Memory.as_str(), "MEMORY");
        assert_eq!(Scheduler.as_str(), "SCHED");
        assert_eq!(Network.as_str(), "NET");
    }
}

// ============================================================================
// MIGRATION GUIDE: From Manual to Autonomous Observability
// ============================================================================
//
// BEFORE (Old way - manual prefixes, manual newlines):
// --------
// fn boot_handler() {
//     let msg = format!("[EARLY SERIAL] x86_64 ap cpu id ready\n");
//     crate::hal::serial::write_raw(&msg);
//     let msg2 = format!("[EARLY SERIAL] core {} online\n", core_id);
//     crate::hal::serial::write_raw(&msg2);
// }
//
// PROBLEMS:
// - Manual "[EARLY SERIAL]" prefix repeated everywhere
// - Manual "\n" appended to every message  
// - No centralized category/section control
// - Compile-time optimization not possible
// - Runtime control requires manual flags in code
//
//
// AFTER (New way - autonomous):
// -------
// fn boot_handler() {
//     let msg = serial_autonomous(Boot, "x86_64 ap cpu id ready");
//     crate::hal::serial::write_raw(&msg);
//     let msg2 = serial_autonomous(Boot, "core {} online", core_id);
//     crate::hal::serial::write_raw(&msg2);
// }
//
// BENEFITS:
// ✓ Prefix auto-generated from category: "[BOOT]"
// ✓ Newline auto-appended
// ✓ Granular category-level control (Boot, Memory, Task, etc.)
// ✓ Runtime overrides via KernelConfig::set_observability_category_enabled()
// ✓ Compile-time optimization via features
// ✓ Consistent namespace across codebase
// ✓ No repeated "[EARLY SERIAL]" strings
// ✓ Dead code elimination when categories disabled
//
//
// COMPILE-TIME CONTROL:
// ---------------------
// Add to Cargo.toml:
//   [features]
//   debug_observability_all = []        # Enable all categories
//   debug_observability_boot = []       # Enable only Boot
//   debug_observability_memory = []     # Enable only Memory
//
// Build with features:
//   $ cargo build --features debug_observability_boot
//   $ cargo build --features debug_observability_all
//
//
// RUNTIME CONTROL:
// ----------------
// At runtime, dynamically enable/disable categories:
//   KernelConfig::set_observability_category_enabled(Boot, Some(true));
//   KernelConfig::set_observability_category_enabled(Memory, Some(false));
//   KernelConfig::set_observability_category_enabled(Scheduler, Some(true));
//
// Check if enabled (respects both compile-time and runtime):
//   if KernelConfig::is_observability_category_enabled(Boot) {
//       // Proceed with boot tracing
//   }
//
//
// NO MORE MANUAL FORMATTING:
// ---------------------------
// Before: let msg = format!("[EARLY SERIAL] allocated 0x1000 bytes\n");
// After:  let msg = serial_autonomous(Memory, "allocated 0x1000 bytes");
//
// Before: let msg = format!("[EARLY SERIAL] addr=0x{:x}\n", frame);
// After:  let msg = serial_autonomous_hex(Memory, "addr", frame);
//
// ============================================================================
