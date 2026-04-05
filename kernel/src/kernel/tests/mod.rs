/// Kernel Core ABI and IPC Parity Tests
///
/// Comprehensive test suites validating Linux application compatibility:
/// - Signal frame delivery against libc expectations
/// - Copy-on-write fork semantics and memory efficiency
/// - Process/session control for multi-process workloads
/// - Robust process teardown and zombie reaping
/// - System V IPC families (semaphores, message queues, shared memory)
/// - Cross-feature fallback for disabled IPC mechanisms
/// - AF_UNIX socket parity for inter-process communication
///
/// Kernel Extended Feature and Compatibility Tests
///
/// Expanded test coverage for production compatibility:
/// - Filesystem backend operations (stat, chmod, extended attributes)
/// - Process and UTS namespace isolation for containers
/// - Socket options for networking (TCP, UDP, IP multicast)
/// - Memory mapping operations (mmap, mprotect, madvise)
/// - Ptrace debugging support for debuggers and strace
///
/// Integration Test Framework & Harness
///
/// Links specifications to executable kernel tests:
/// - Mock syscall layer for isolated testing
/// - Multi-process test execution framework
/// - Result collection and reporting
/// - State management and validation

// Core Tests - ABI and IPC Parity (Documentation Specifications)
#[cfg(all(test, target_os = "none"))]
mod signal_frame_parity;

#[cfg(all(test, target_os = "none"))]
mod syscall_semantic_parity;

#[cfg(all(test, target_os = "none"))]
mod fork_cow_semantics;

#[cfg(all(test, target_os = "none"))]
mod process_session_control;

#[cfg(all(test, target_os = "none"))]
mod process_teardown_semantics;

#[cfg(all(test, target_os = "none"))]
mod process_signal_race_compat;

#[cfg(all(test, target_os = "none"))]
mod time_abi_parity;

#[cfg(all(test, target_os = "none"))]
mod sysv_ipc_parity;

#[cfg(all(test, target_os = "none"))]
mod cross_feature_ipc_fallback;

#[cfg(all(test, target_os = "none"))]
mod af_unix_parity;

// P0/P1 Test Integration Framework (117 P0 + 127 P1 test specifications)
#[cfg(all(test, target_os = "none"))]
mod p0_integration_harness;

// P0 Process/Session Control - Implementation Details
#[cfg(all(test, target_os = "none"))]
mod p0_process_session_control_impl;

// Extended Tests - Features and Compatibility (Documentation Specifications)
#[cfg(all(test, target_os = "none"))]
mod fs_backend_parity;

#[cfg(all(test, target_os = "none"))]
mod pid_uts_namespace_parity;

#[cfg(all(test, target_os = "none"))]
mod socket_options_parity;

#[cfg(all(test, target_os = "none"))]
mod memory_mapping_parity;

#[cfg(all(test, target_os = "none"))]
mod ptrace_debugging_parity;

#[cfg(all(test, target_os = "none"))]
mod proc_sysctl_consistency_parity;

#[cfg(all(test, target_os = "none"))]
mod fd_edge_case_parity;

// Integration Test Framework & Harness
#[cfg(all(test, target_os = "none"))]
mod integration_harness;

// Core Integration Tests (Executable against harness)
#[cfg(all(test, target_os = "none"))]
mod signal_frame_integration;

#[cfg(all(test, target_os = "none"))]
mod fork_cow_integration;

#[cfg(all(test, target_os = "none"))]
mod process_wait_integration;

// Extended Integration Tests (Executable against harness)
#[cfg(all(test, target_os = "none"))]
mod filesystem_integration;

#[cfg(all(test, target_os = "none"))]
mod socket_options_integration;

#[cfg(all(test, target_os = "none"))]
mod ptrace_integration;

#[cfg(all(test, target_os = "none"))]
mod proc_sysctl_integration;

