//! Emits #[cfg(...)] check and compile flags.

use super::config_types::Config;

pub fn emit_check_cfgs() {
    println!(
        "cargo:rustc-check-cfg=cfg(param_scheduler, values(\"RoundRobin\", \"CFS\", \"EDF\", \"FIFO\", \"Cooperative\", \"Lottery\", \"WeightedRoundRobin\", \"LIFO\", \"MLFQ\", \"Idle\", \"MuQSS\", \"EEVDF\", \"RealTimeHard\", \"RealTimeSoft\", \"Batch\", \"UserSpace\"))"
    );
    println!(
        "cargo:rustc-check-cfg=cfg(param_allocator, values(\"Bump\", \"LinkedListAllocator\", \"Slab\", \"Buddy\", \"PoolAllocator\"))"
    );
    println!(
        "cargo:rustc-check-cfg=cfg(param_dispatcher, values(\"DirectForwarding\", \"Buffered\", \"Vectored\", \"Managed\"))"
    );
    println!(
        "cargo:rustc-check-cfg=cfg(param_ipc, values(\"ZeroCopy\", \"MessagePassing\", \"SignalOnly\", \"Pipes\", \"RingBuffer\", \"Futex\"))"
    );
    println!(
        "cargo:rustc-check-cfg=cfg(param_security_monitor, values(\"NullMonitor\", \"AccessControlList\", \"ObjectCapability\", \"SeL4_Style\"))"
    );
    println!("cargo:rustc-check-cfg=cfg(param_ring_level, values(\"Ring0\", \"Ring3\"))");
    println!(
        "cargo:rustc-check-cfg=cfg(param_log_level, values(\"Error\", \"Warn\", \"Info\", \"Debug\", \"Trace\"))"
    );
    println!(
        "cargo:rustc-check-cfg=cfg(param_boundary_mode, values(\"Strict\", \"Balanced\", \"Compat\"))"
    );
    println!(
        "cargo:rustc-check-cfg=cfg(feature, values(\"paging\", \"guardian_pages\", \"smap_smep\", \"nx_bit\", \"telemetry\", \"security_null\", \"security_acl\", \"security_capabilities\", \"security_sel4\", \"rtos_strict\", \"rtos_posix\"))"
    );
}

pub fn emit_compile_cfgs(config: &Config) {
    println!(
        "cargo:rustc-cfg=param_scheduler=\"{}\"",
        config.scheduler.strategy
    );
    println!(
        "cargo:rustc-cfg=param_allocator=\"{}\"",
        config.memory.allocator
    );
    println!(
        "cargo:rustc-cfg=param_dispatcher=\"{}\"",
        config.dispatcher.strategy
    );
    println!("cargo:rustc-cfg=param_ipc=\"{}\"", config.ipc.mechanism);
    println!(
        "cargo:rustc-cfg=param_security_monitor=\"{}\"",
        config.security.monitor
    );
    println!(
        "cargo:rustc-cfg=param_ring_level=\"{}\"",
        config.security.ring_level
    );
    println!(
        "cargo:rustc-cfg=param_log_level=\"{}\"",
        config.telemetry.log_level
    );
    println!(
        "cargo:rustc-cfg=param_boundary_mode=\"{}\"",
        config.library.boundary_mode
    );

    if config.memory.paging {
        println!("cargo:rustc-cfg=feature=\"paging\"");
    }
    if config.memory.guardian_pages {
        println!("cargo:rustc-cfg=feature=\"guardian_pages\"");
    }
    if config.security.nx_bit {
        println!("cargo:rustc-cfg=feature=\"nx_bit\"");
    }
    if config.security.smap_smep {
        println!("cargo:rustc-cfg=feature=\"smap_smep\"");
    }
    if config.telemetry.enabled {
        println!("cargo:rustc-cfg=feature=\"telemetry\"");
    }
    if config.security.ring_level == "Ring3" {
        println!("cargo:rustc-cfg=feature=\"ring_protection\"");
    }

    match config.security.monitor.as_str() {
        "NullMonitor" => println!("cargo:rustc-cfg=feature=\"security_null\""),
        "AccessControlList" => println!("cargo:rustc-cfg=feature=\"security_acl\""),
        "ObjectCapability" => println!("cargo:rustc-cfg=feature=\"security_capabilities\""),
        "SeL4_Style" => println!("cargo:rustc-cfg=feature=\"security_sel4\""),
        _ => {
            println!(
                "cargo:warning=Unknown security monitor '{}', defaulting to NullMonitor",
                config.security.monitor
            );
            println!("cargo:rustc-cfg=feature=\"security_null\"");
        }
    }

    if config.rtos.strict_profile_enabled {
        println!("cargo:rustc-cfg=feature=\"rtos_strict\"");
    }
    if config.rtos.posix_compat_enabled {
        println!("cargo:rustc-cfg=feature=\"rtos_posix\"");
    }
}
