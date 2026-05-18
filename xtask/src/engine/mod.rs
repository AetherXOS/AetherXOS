pub mod audit;
pub mod cache;
pub mod context;
pub mod controller;
pub mod dag;
pub mod debug;
pub mod distro;
pub mod docs;
pub mod filesystem;
pub mod graph;
pub mod help;
pub mod hook;
pub mod iso;
pub mod macros;
pub mod net;
pub mod operations;
pub mod parallel;
pub mod pipeline;
pub mod profile;
pub mod profiler;
pub mod qemu;
pub mod resource_audit;
pub mod snapshot;
pub mod staging;
pub mod state;
pub mod task;
pub mod telemetry;
pub mod workflow;

// High-level re-exports for a clean public API
pub use context::ExecutionContext;
pub use debug::DebugBridgeTask;
pub use docs::DocsGenerateTask;
#[allow(unused_imports)]
pub use hook::HookTask;
pub use net::DownloadTask;
pub use operations::Op;
pub use pipeline::Pipeline;
pub use profile::BuildProfile;
#[allow(unused_imports)]
pub use qemu::QemuRunTask;
pub use resource_audit::ResourceAuditTask;
pub use staging::StagingArea;
pub use state::EngineState;
pub use task::{Task, TaskStatus};
