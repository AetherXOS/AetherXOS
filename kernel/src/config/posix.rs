//! POSIX Subsystem Configuration
//!
//! Contains tuning parameters and limits for POSIX-compliant features
//! such as pipes, signals, and shared memory.

/// Number of times to spin while waiting for pipe I/O before suspending the task.
/// Higher values improve throughput for small, frequent transfers at the cost of CPU.
pub const PIPE_IO_SPIN_BUDGET: usize = 1024;

/// Default capacity for the POSIX signal queue per task.
pub const SIGNAL_QUEUE_CAPACITY: usize = 32;

/// Maximum number of POSIX message queues system-wide.
pub const MAX_POSIX_MQS: usize = 256;

/// Default size for POSIX shared memory segments if not specified.
pub const DEFAULT_SHM_SEGMENT_SIZE: usize = 4096 * 16;
