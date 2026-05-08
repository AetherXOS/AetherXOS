/// Security extension interfaces.
/// 
/// Audit trail logging, threat detection, and security event tracking.

use crate::interfaces::KernelResult;
use alloc::string::String;
use alloc::vec::Vec;

/// Security event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventType {
    /// File access (read/write/execute)
    FileAccess,
    /// Process execution
    ProcessExec,
    /// Process termination
    ProcessExit,
    /// Authentication attempt
    Authentication,
    /// Authorization decision (allow/deny)
    Authorization,
    /// System call invocation
    SystemCall,
    /// Device access
    DeviceAccess,
    /// Network connection
    NetworkAccess,
    /// Policy violation
    PolicyViolation,
    /// Security module event
    SecurityModule,
    /// Custom event
    Custom,
}

/// Audit event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    /// Informational
    Info,
    /// Warning
    Warning,
    /// Error
    Error,
    /// Critical security issue
    Critical,
}

/// Audit event record
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Unique event ID
    pub event_id: u64,
    /// Timestamp (nanoseconds since boot)
    pub timestamp_ns: u64,
    /// Event type
    pub event_type: AuditEventType,
    /// Severity level
    pub severity: AuditSeverity,
    /// Process ID
    pub pid: u32,
    /// User ID
    pub uid: u32,
    /// Event description
    pub description: String,
    /// Additional context (e.g., file path, syscall number)
    pub context: Option<String>,
    /// Was action allowed?
    pub allowed: bool,
}

/// Filter for audit log queries
#[derive(Debug, Clone)]
pub struct AuditFilter {
    /// Filter by event type
    pub event_type: Option<AuditEventType>,
    /// Filter by minimum severity
    pub min_severity: Option<AuditSeverity>,
    /// Filter by UID
    pub uid: Option<u32>,
    /// Filter by PID
    pub pid: Option<u32>,
    /// Filter by time range (start, end nanoseconds)
    pub time_range: Option<(u64, u64)>,
    /// Filter by denied actions only
    pub denied_only: bool,
}

/// Trait for audit logging
pub trait AuditLogger {
    /// Log a security event
    fn log_event(&mut self, event: AuditEvent) -> KernelResult<()>;

    /// Query events with filter
    fn query_events(&self, filter: &AuditFilter) -> KernelResult<Vec<AuditEvent>>;

    /// Get event by ID
    fn get_event(&self, event_id: u64) -> KernelResult<Option<AuditEvent>>;

    /// Clear old events (before timestamp)
    fn clear_old_events(&mut self, before_ns: u64) -> KernelResult<usize>;

    /// Get total events logged
    fn total_events(&self) -> u64;

    /// Get denied events count
    fn denied_events_count(&self) -> u64;

    /// Export events to buffer
    fn export_events(&self, format: ExportFormat) -> KernelResult<Vec<u8>>;
}

/// Event export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Text format (one event per line)
    Text,
    /// JSON format
    Json,
    /// CSV format
    Csv,
    /// Binary format
    Binary,
}

/// Threat level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    /// Normal operation
    Normal = 0,
    /// Suspicious behavior
    Suspicious = 1,
    /// Potential threat
    Threat = 2,
    /// Active attack
    Attack = 3,
}

/// Threat detection metrics
#[derive(Debug, Clone)]
pub struct ThreatMetrics {
    /// Current threat level
    pub current_level: ThreatLevel,
    /// Denied access attempts in last period
    pub denied_attempts: u32,
    /// Failed authentications
    pub auth_failures: u32,
    /// Policy violations
    pub policy_violations: u32,
    /// Suspicious processes
    pub suspicious_processes: u32,
    /// Detection score (0-100)
    pub detection_score: u8,
}

/// Trait for threat detection
pub trait ThreatDetector {
    /// Analyze events for threats
    fn analyze_events(&self, time_window_ns: u64) -> KernelResult<ThreatMetrics>;

    /// Get current threat level
    fn current_threat_level(&self) -> ThreatLevel;

    /// Trigger alert if threat detected
    fn check_and_alert(&mut self, event: &AuditEvent) -> KernelResult<Option<String>>;

    /// Set threat detection sensitivity (0-100, higher = more sensitive)
    fn set_sensitivity(&mut self, sensitivity: u8) -> KernelResult<()>;

    /// Get threat detection sensitivity
    fn get_sensitivity(&self) -> u8;
}

/// Trait for security incident response
pub trait IncidentResponder {
    /// Report security incident
    fn report_incident(&mut self, event: &AuditEvent, action: ResponseAction) -> KernelResult<()>;

    /// Get incident log
    fn get_incidents(&self) -> Vec<SecurityIncident>;

    /// Clear incident
    fn clear_incident(&mut self, incident_id: u32) -> KernelResult<()>;
}

/// Security incident
#[derive(Debug, Clone)]
pub struct SecurityIncident {
    /// Incident ID
    pub incident_id: u32,
    /// Triggering audit event
    pub trigger_event: AuditEvent,
    /// Response action taken
    pub response: ResponseAction,
    /// Incident severity
    pub severity: AuditSeverity,
}

/// Response action for security incidents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseAction {
    /// Log only
    Log,
    /// Alert administrators
    Alert,
    /// Terminate process
    Terminate,
    /// Suspend process
    Suspend,
    /// Block user/process
    Block,
    /// System lockdown
    Lockdown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_severity_ordering() {
        assert!(AuditSeverity::Info < AuditSeverity::Warning);
        assert!(AuditSeverity::Warning < AuditSeverity::Error);
        assert!(AuditSeverity::Error < AuditSeverity::Critical);
    }

    #[test]
    fn test_threat_level_ordering() {
        assert!(ThreatLevel::Normal < ThreatLevel::Suspicious);
        assert!(ThreatLevel::Suspicious < ThreatLevel::Threat);
        assert!(ThreatLevel::Threat < ThreatLevel::Attack);
    }

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent {
            event_id: 1,
            timestamp_ns: 1000,
            event_type: AuditEventType::FileAccess,
            severity: AuditSeverity::Info,
            pid: 100,
            uid: 1000,
            description: "File read".into(),
            context: Some("/etc/passwd".into()),
            allowed: true,
        };

        assert_eq!(event.pid, 100);
        assert!(event.allowed);
    }

    #[test]
    fn test_threat_metrics() {
        let metrics = ThreatMetrics {
            current_level: ThreatLevel::Threat,
            denied_attempts: 10,
            auth_failures: 5,
            policy_violations: 3,
            suspicious_processes: 2,
            detection_score: 75,
        };

        assert_eq!(metrics.current_level, ThreatLevel::Threat);
        assert!(metrics.detection_score > 50);
    }
}
