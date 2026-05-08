//! Trait bridge for the existing hardware abstraction contracts.

pub use crate::interfaces::hardware::{
    HardwareAbstraction, InterruptController, PciController, PerformanceProfile, PortIo,
    SerialDevice, Timer,
};
pub use crate::interfaces::scheduler::{Scheduler, SchedulerAction};
pub use crate::interfaces::task::{Context, KernelTask, ProcessId, TaskId, TaskState};