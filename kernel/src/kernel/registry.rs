//! Service Registry - Type-safe dependency injection for the AetherXOS kernel.
//!
//! This module provides a central registry where core services (Scheduler, VFS, Net, etc.)
//! can be registered during boot and retrieved by their trait type.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::any::{Any, TypeId};

/// Error types for the Registry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    NotFound,
    AlreadyRegistered,
    TypeMismatch,
    ServiceMissing,
}

/// A central registry for kernel services.
/// IrqSafeMutex-protected map for thread-safe access across cores and IRQs.
pub struct ServiceRegistry {
    services: crate::kernel::sync::IrqSafeMutex<BTreeMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl ServiceRegistry {
    /// Create a new, empty registry
    pub const fn new() -> Self {
        Self {
            services: crate::kernel::sync::IrqSafeMutex::new(BTreeMap::new()),
        }
    }

    /// Register a service with the registry
    pub fn register<T: 'static + Send + Sync>(&self, service: Arc<T>) -> Result<(), RegistryError> {
        let type_id = TypeId::of::<T>();
        let mut services = self.services.lock();
        
        if services.contains_key(&type_id) {
            return Err(RegistryError::AlreadyRegistered);
        }
        
        services.insert(type_id, service);
        Ok(())
    }

    /// Retrieve a service from the registry by its type.
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();
        let services = self.services.lock();
        
        services.get(&type_id).and_then(|s| s.clone().downcast::<T>().ok())
    }

    /// Check if a service is registered
    pub fn has<T: 'static + Send + Sync>(&self) -> bool {
        self.services.lock().contains_key(&TypeId::of::<T>())
    }
}

/// Global service registry instance
pub static GLOBAL_REGISTRY: ServiceRegistry = ServiceRegistry::new();

/// Convenience macro for registering a service
#[macro_export]
macro_rules! register_service {
    ($service:expr) => {
        $crate::kernel::registry::GLOBAL_REGISTRY.register($service)
    };
}

/// Convenience macro for getting a service
#[macro_export]
macro_rules! get_service {
    ($t:ty) => {
        $crate::kernel::registry::GLOBAL_REGISTRY.get::<$t>()
    };
}
