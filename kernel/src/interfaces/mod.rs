pub mod cpu;
// Core trait modules
pub mod boot;
pub mod device;
pub mod platform;
pub mod runtime;
// Subsystem trait modules
pub mod dispatcher;
pub mod error;
pub mod governance;
pub mod hardware;
pub mod ipc;
pub mod memory;
pub mod scheduler;
pub mod security;
pub mod task;
// Extension trait modules
pub mod memory_ext;
pub mod scheduler_ext;
pub mod security_ext;
pub mod vfs_ext;

// Re-exports - Core
pub use boot::{BootManager, BootStage, BootSubsystem};
pub use device::{DeviceManager, DeviceRegistry};
pub use platform::{Platform, PlatformServices};
pub use runtime::{RuntimeManager, RuntimeState};

// Re-exports - Subsystems
pub use dispatcher::Dispatcher;
pub use error::{KernelError, KernelResult};
pub use governance::{Governance, SystemState};
pub use hardware::{HardwareAbstraction, InterruptController, PciController, PerformanceProfile, PortIo, SerialDevice};
pub use ipc::IpcChannel;
pub use memory::{HeapAllocator, PageAllocator, PAGE_SIZE_1G, PAGE_SIZE_2M, PAGE_SIZE_4K};
pub use scheduler::{Scheduler, SchedulerAction};
pub use security::{
    cap_flags, ResourceKind, ResourceLimits, SecurityAction, SecurityContext, SecurityLevel,
    SecurityMonitor, SecurityVerdict,
};
pub use task::{Context, KernelTask, ProcessId, TaskId, TaskState};
