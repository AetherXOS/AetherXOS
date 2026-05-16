pub mod task;
pub mod pipeline;
pub mod context;
pub mod filesystem;
pub mod net;
pub mod iso;
pub mod qemu;
pub mod distro;
pub mod staging;
pub mod operations;
pub mod state;
pub mod workflow;
pub mod parallel;
pub mod audit;
pub mod telemetry;
pub mod debug;
pub mod docs;
pub mod hook;
pub mod graph;
pub mod help;
pub mod resource_audit;
pub mod dag;
pub mod profile;
pub mod cache;
pub mod macros;
pub mod snapshot;
pub mod profiler;
pub mod controller;

// High-level re-exports for a clean public API
pub use task::{Task, TaskStatus};
pub use pipeline::Pipeline;
pub use context::ExecutionContext;
pub use state::EngineState;
pub use staging::StagingArea;
pub use operations::Op;
pub use net::DownloadTask;
#[allow(unused_imports)]
pub use qemu::QemuRunTask;
pub use debug::DebugBridgeTask;
pub use docs::DocsGenerateTask;
pub use resource_audit::ResourceAuditTask;
#[allow(unused_imports)]
pub use hook::HookTask;
pub use profile::BuildProfile;
