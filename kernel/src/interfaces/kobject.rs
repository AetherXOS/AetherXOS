//! KObject - The unified interface for all kernel resources.
//!
//! Every major resource in the AetherXOS kernel implements this trait to provide
//! a consistent way to handle identity, security, and lifecycle.

use core::any::Any;
use crate::interfaces::security::SecurityContext;

/// Categories of kernel objects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Task,
    Process,
    File,
    Directory,
    Socket,
    IpcChannel,
    Device,
    MemoryRegion,
    Namespace,
}

/// The base trait for all kernel objects.
pub trait KObject: Any + Send + Sync {
    /// Get the unique ID of the object.
    fn id(&self) -> u64;
    
    /// Get the kind of the object.
    fn kind(&self) -> ObjectKind;
    
    /// Get the security context associated with this object.
    fn security_context(&self) -> Option<&SecurityContext>;

    /// Cast to Any for downcasting support.
    fn as_any(&self) -> &dyn Any;
}

/// A reference-counted handle to a kernel object.
pub type ObjectRef = alloc::sync::Arc<dyn KObject>;
