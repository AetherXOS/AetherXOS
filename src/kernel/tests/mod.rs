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
#[cfg(test)]
mod signal_frame_parity;

#[cfg(test)]
mod fork_cow_semantics;

#[cfg(test)]
mod process_session_control;

#[cfg(test)]
mod process_teardown_semantics;

#[cfg(test)]
mod sysv_ipc_parity;

#[cfg(test)]
mod cross_feature_ipc_fallback;

#[cfg(test)]
mod af_unix_parity;

// P0/P1 Test Integration Framework (117 P0 + 127 P1 test specifications)
#[cfg(test)]
mod p0_integration_harness;

// P0 Process/Session Control - Implementation Details
#[cfg(test)]
mod p0_process_session_control_impl;

// Extended Tests - Features and Compatibility (Documentation Specifications)
#[cfg(test)]
mod fs_backend_parity;

#[cfg(test)]
mod pid_uts_namespace_parity;

#[cfg(test)]
mod socket_options_parity;

#[cfg(test)]
mod memory_mapping_parity;

#[cfg(test)]
mod ptrace_debugging_parity;

#[cfg(test)]
mod proc_sysctl_consistency_parity;

// Integration Test Framework & Harness
#[cfg(test)]
mod integration_harness;

// Core Integration Tests (Executable against harness)
#[cfg(test)]
mod signal_frame_integration;

#[cfg(test)]
mod fork_cow_integration;

#[cfg(test)]
mod process_wait_integration;

// Extended Integration Tests (Executable against harness)
#[cfg(test)]
mod filesystem_integration;

#[cfg(test)]
mod socket_options_integration;

#[cfg(test)]
mod ptrace_integration;

#[cfg(test)]
mod proc_sysctl_integration;

