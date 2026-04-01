// Integration Example: Boot Module Using Autonomous Observability
//
// This demonstrates how to integrate the new autonomous observability system
// into existing boot code with minimal changes.

#[cfg(test)]
mod boot_observability_examples {
    use crate::config::*;

    /// Example: x86_64 AP bootstrap sequence with autonomous serials
    /// 
    /// Before:
    /// ```ignore
    /// for ap_id in 0..num_aps {
    ///     let msg = format!("[EARLY SERIAL] x86_64 ap {} starting\n", ap_id);
    ///     serial::write_raw(&msg);
    /// }
    /// ```
    ///
    /// After:
    /// ```ignore
    /// for ap_id in 0..num_aps {
    ///     let msg = serial_autonomous(Boot, &format!("x86_64 ap {} starting", ap_id));
    ///     serial::write_raw(&msg);
    /// }
    /// ```
    #[test_case]
    fn example_ap_bootstrap() {
        // Enable Boot category for this sequence
        KernelConfig::set_observability_category_enabled(ObservabilityCategory::Boot, Some(true));

        // Check if enabled before emitting
        if KernelConfig::is_observability_category_enabled(ObservabilityCategory::Boot) {
            for ap_id in 0..2 {
                let msg = serial_autonomous(
                    ObservabilityCategory::Boot,
                    &format!("x86_64 ap {} starting", ap_id),
                );
                assert!(msg.contains(&format!("[BOOT] x86_64 ap {} starting\n", ap_id)));
            }
        }
    }

    /// Example: Memory initialization with granular category control
    ///
    /// Shows how to control memory observability independently from boot
    #[test_case]
    fn example_memory_init() {
        // Enable memory observability explicitly
        KernelConfig::set_observability_category_enabled(ObservabilityCategory::Memory, Some(true));
        // But keep boot observability at default (via compile-time features)

        let frame_count = 1000;
        if KernelConfig::is_observability_category_enabled(ObservabilityCategory::Memory) {
            let msg = serial_autonomous_hex(
                ObservabilityCategory::Memory,
                "frames_allocated",
                frame_count as u64,
            );
            assert_eq!(msg, "[MEMORY] frames_allocated=0x3e8\n");
        }
    }

    /// Example: Fault handler with priority gating
    ///
    /// Fault observability can be disabled independently to reduce noise
    #[test_case]
    fn example_fault_handling() {
        // During normal boot, we might disable fault tracing
        KernelConfig::set_observability_category_enabled(ObservabilityCategory::Fault, Some(false));

        // Code only emits if enabled
        if KernelConfig::is_observability_category_enabled(ObservabilityCategory::Fault) {
            // This block won't execute since we disabled Fault category
            panic!("Should not reach here");
        }

        // Now enable it for debugging
        KernelConfig::set_observability_category_enabled(ObservabilityCategory::Fault, Some(true));

        if KernelConfig::is_observability_category_enabled(ObservabilityCategory::Fault) {
            let msg = serial_autonomous_hex(ObservabilityCategory::Fault, "page_fault", 0x4000);
            assert_eq!(msg, "[FAULT] page_fault=0x4000\n");
        }
    }

    /// Example: Category reset (for testing)
    #[test_case]
    fn example_category_reset() {
        use alloc::format;

        // Set all categories
        KernelConfig::set_observability_category_enabled(ObservabilityCategory::Boot, Some(true));
        KernelConfig::set_observability_category_enabled(ObservabilityCategory::Memory, Some(true));

        // Verify they're set
        assert!(
            KernelConfig::is_observability_category_enabled(ObservabilityCategory::Boot)
                || !cfg!(not(any(
                    feature = "debug_observability_boot",
                    feature = "debug_observability_all"
                )))
        );

        // Reset by setting to None (revert to compile-time default)
        KernelConfig::set_observability_category_enabled(ObservabilityCategory::Boot, None);
        KernelConfig::set_observability_category_enabled(ObservabilityCategory::Memory, None);
    }

    /// Example: Formatted messages without manual newlines
    #[test_case]
    fn example_formatted_messages() {
        // Old way:
        // let msg = format!("[EARLY SERIAL] cpu {} online at {}ms\n", cpu_id, timestamp);

        // New way - using standard format!() with autonomous prefix:
        let cpu_id = 0;
        let timestamp = 1234;
        let msg_formatted = format!(
            "{} cpu {} online at {}ms\n",
            ObservabilityCategory::Boot.as_str(),
            cpu_id,
            timestamp
        );

        // Or more concisely, prepare with autonomous:
        let msg_autonomous = serial_autonomous(
            ObservabilityCategory::Boot,
            &format!("cpu {} online at {}ms", cpu_id, timestamp),
        );

        // Both produce: "[BOOT] cpu 0 online at 1234ms\n"
        assert_eq!(msg_autonomous, format!("[BOOT] {} online at {}ms\n", "cpu 0", timestamp));
    }

    /// Example: Conditional compilation with features
    ///
    /// When compiled with `debug_observability_boot` feature,
    /// this code path is optimizable, when not compiled, it may be dead code.
    #[test_case]
    fn example_feature_gating() {
        // This check is constant at compile-time
        if is_category_enabled_compile_time(ObservabilityCategory::Boot) {
            // When debug_observability_boot is enabled, this is true
            // When disabled, compiler can eliminate this block
            let msg = serial_autonomous(ObservabilityCategory::Boot, "feature gated");
            assert_eq!(msg, "[BOOT] feature gated\n");
        }
    }
}

// This would be part of the actual boot code:
// 
// fn x86_64_bootstrap() {
//     // Enable observability categories for this phase
//     KernelConfig::set_observability_category_enabled(Boot, Some(true));
//     KernelConfig::set_observability_category_enabled(Core, Some(true));
//
//     // Regular boot code...
//     if KernelConfig::is_observability_category_enabled(Boot) {
//         let msg = serial_autonomous(Boot, "x86_64 bootstrap started");
//         hal::serial::SERIAL1.write_str(&msg);
//     }
//
//     // Memory setup...
//     if KernelConfig::is_observability_category_enabled(Memory) {
//         let msg = serial_autonomous_hex(Memory, "memory_base", 0x1000000);
//         hal::serial::SERIAL1.write_str(&msg);
//     }
//
//     // Scheduler setup...
//     if KernelConfig::is_observability_category_enabled(Scheduler) {
//         let msg = serial_autonomous(Scheduler, "scheduler initialized");
//         hal::serial::SERIAL1.write_str(&msg);
//     }
// }

