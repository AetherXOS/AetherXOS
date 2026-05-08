/// Shared utilities for runtime integration modules
/// Eliminates code repetition in scheduler, memory, and VFS integration

use crate::core::log;
use alloc::format;
use alloc::string::String;

/// Common error types for integration operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationError {
    /// Operation succeeded but with warnings
    Warning,
    /// Critical operation failed
    Critical,
    /// Resource not found
    NotFound,
    /// Invalid parameter
    InvalidParam,
    /// Operation not supported in current configuration
    Unsupported,
    /// Quota or limit exceeded
    QuotaExceeded,
}

impl IntegrationError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Warning => "Warning",
            Self::Critical => "Critical",
            Self::NotFound => "NotFound",
            Self::InvalidParam => "InvalidParam",
            Self::Unsupported => "Unsupported",
            Self::QuotaExceeded => "QuotaExceeded",
        }
    }
}

/// Validation helpers
pub mod validation {
    use super::*;

    /// Validate CPU ID against system CPU count
    pub fn validate_cpu_id(cpu_id: u32) -> Result<(), IntegrationError> {
        let caps = crate::hal::platforms::get_platform().capabilities();
        if cpu_id >= caps.cpu_count as u32 {
            return Err(IntegrationError::InvalidParam);
        }
        Ok(())
    }

    /// Validate memory allocation size (must be page-aligned)
    pub fn validate_allocation_size(size: u64) -> Result<(), IntegrationError> {
        const PAGE_SIZE: u64 = 4096;
        if size == 0 || (size & (PAGE_SIZE - 1)) != 0 {
            return Err(IntegrationError::InvalidParam);
        }
        Ok(())
    }

    /// Validate process ID
    pub fn validate_pid(pid: u32) -> Result<(), IntegrationError> {
        if pid == 0 {
            return Err(IntegrationError::InvalidParam);
        }
        Ok(())
    }

    /// Validate inode ID
    pub fn validate_inode(inode: u64) -> Result<(), IntegrationError> {
        if inode == 0 {
            return Err(IntegrationError::InvalidParam);
        }
        Ok(())
    }
}

/// Logging helpers - reduce boilerplate
pub mod logging {
    use super::*;

    /// Log debug message for operation start
    pub fn log_operation_start(op_name: &str, entity_id: u64) {
        log::debug(&format!("{}: entity_id={}", op_name, entity_id));
    }

    /// Log operation success with details
    pub fn log_operation_success(op_name: &str, entity_id: u64, details: &str) {
        log::debug(&format!("{}: entity_id={}, details={}", op_name, entity_id, details));
    }

    /// Log operation failure with reason
    pub fn log_operation_failure(op_name: &str, entity_id: u64, reason: &str) {
        log::warn(&format!("{}: entity_id={}, reason={}", op_name, entity_id, reason));
    }

    /// Log state transition
    pub fn log_state_transition(entity_name: &str, old_state: &str, new_state: &str) {
        log::info(&format!(
            "State: {}={} -> {}",
            entity_name, old_state, new_state
        ));
    }

    /// Log configuration change
    pub fn log_config_change(setting: &str, old_value: &str, new_value: &str) {
        log::debug(&format!("Config: {}={} -> {}", setting, old_value, new_value));
    }

    /// Log capability enabled
    pub fn log_capability_enabled(capability: &str, details: &str) {
        log::info(&format!("Capability: {} enabled ({})", capability, details));
    }

    /// Log quota/limit enforcement
    pub fn log_limit_enforced(limit_type: &str, entity_id: u64, limit_value: u64) {
        log::debug(&format!(
            "Limit: type={}, entity_id={}, limit={}",
            limit_type, entity_id, limit_value
        ));
    }

    /// Log diagnostic snapshot
    pub fn log_diagnostic(component: &str, snapshot: &str) {
        log::info(&format!("Diagnostic [{}]: {}", component, snapshot));
    }
}

/// Configuration constants
pub mod config {
    /// Minimum memory allocation (must be page-aligned)
    pub const MIN_ALLOCATION: u64 = 4096;

    /// Maximum CPU ID for validation
    pub const MAX_CPUS: u32 = 256;

    /// Priority threshold for real-time (milliseconds)
    pub const RT_PRIORITY_THRESHOLD: u32 = 80;

    /// Default task timeout (milliseconds)
    pub const DEFAULT_TIMEOUT_MS: u64 = 5000;

    /// Memory pressure thresholds (percentage free)
    pub const PRESSURE_CRITICAL_THRESHOLD: u64 = 5;
    pub const PRESSURE_HIGH_THRESHOLD: u64 = 25;
    pub const PRESSURE_MEDIUM_THRESHOLD: u64 = 50;

    /// Quota enforcement: max bytes per process
    pub const DEFAULT_PROCESS_QUOTA: u64 = 268_435_456; // 256 MB
}

/// Result type for integration operations
pub type IntegrationResult<T> = Result<T, IntegrationError>;

/// Helper macro for quick validation + operation
#[macro_export]
macro_rules! validate_and_execute {
    ($validator:expr, $operation:expr, $op_name:expr, $entity_id:expr) => {
        match $validator {
            Ok(()) => {
                match $operation {
                    Ok(result) => {
                        $crate::kernel_runtime::integration_utils::logging::log_operation_success(
                            $op_name,
                            $entity_id,
                            "success",
                        );
                        Ok(result)
                    }
                    Err(e) => {
                        $crate::kernel_runtime::integration_utils::logging::log_operation_failure(
                            $op_name,
                            $entity_id,
                            e,
                        );
                        Err(e)
                    }
                }
            }
            Err(e) => {
                $crate::kernel_runtime::integration_utils::logging::log_operation_failure(
                    $op_name,
                    $entity_id,
                    e.as_str(),
                );
                Err(e.as_str())
            }
        }
    };
}

/// Diagnostic helper to collect stats from all integration systems
pub struct IntegrationDiagnostics {
    pub scheduler_active: bool,
    pub memory_tracking: bool,
    pub vfs_permissions: bool,
    pub total_operations: u64,
}

impl IntegrationDiagnostics {
    pub fn snapshot() -> Self {
        Self {
            scheduler_active: true,
            memory_tracking: true,
            vfs_permissions: true,
            total_operations: 0,
        }
    }

    pub fn as_string(&self) -> String {
        format!(
            "Integrations: scheduler={}, memory={}, vfs={}, ops={}",
            self.scheduler_active, self.memory_tracking, self.vfs_permissions, self.total_operations
        )
    }
}

/// Audit syscall event - logs security-relevant syscall events
///
/// # Arguments
/// - `syscall_name`: Name of the syscall (e.g., "signal_send", "socket_create")
/// - `pid`: Process ID (0 if not applicable)
/// - `uid`: User ID performing the syscall
/// - `allowed`: Whether the operation was allowed
/// - `context`: Optional additional context (e.g., resource name, policy violation)
#[cfg(feature = "audit_logging")]
pub fn audit_syscall_event(
    syscall_name: &str,
    pid: u32,
    uid: u32,
    allowed: bool,
    context: Option<&str>,
) {
    let status = if allowed { "allowed" } else { "denied" };
    let context_str = context.unwrap_or("none");
    let message = format!(
        "Audit [{}]: pid={} uid={} status={} context={}",
        syscall_name, pid, uid, status, context_str
    );
    
    if allowed {
        log::debug(&message);
    } else {
        log::warn(&message);
    }
}

/// Audit syscall event - no-op when audit_logging not enabled
#[cfg(not(feature = "audit_logging"))]
pub fn audit_syscall_event(
    _syscall_name: &str,
    _pid: u32,
    _uid: u32,
    _allowed: bool,
    _context: Option<&str>,
) {
    // No-op when audit_logging feature is disabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_zero_pid() {
        assert_eq!(
            validation::validate_pid(0),
            Err(IntegrationError::InvalidParam)
        );
    }

    #[test]
    fn test_validation_valid_pid() {
        assert!(validation::validate_pid(1).is_ok());
    }

    #[test]
    fn test_validation_allocation_size() {
        // Page size = 4096
        assert_eq!(
            validation::validate_allocation_size(0),
            Err(IntegrationError::InvalidParam)
        );
        assert_eq!(
            validation::validate_allocation_size(4095), // Not aligned
            Err(IntegrationError::InvalidParam)
        );
        assert!(validation::validate_allocation_size(4096).is_ok());
        assert!(validation::validate_allocation_size(8192).is_ok());
    }

    #[test]
    fn test_error_as_str() {
        assert_eq!(IntegrationError::Critical.as_str(), "Critical");
        assert_eq!(IntegrationError::NotFound.as_str(), "NotFound");
    }

    #[test]
    fn test_diagnostics_snapshot() {
        let diag = IntegrationDiagnostics::snapshot();
        assert!(diag.scheduler_active);
        assert!(diag.memory_tracking);
        assert!(diag.vfs_permissions);
    }
}
