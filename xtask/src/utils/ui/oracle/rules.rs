use std::path::Path;

#[derive(Debug, Clone)]
pub struct SafetyIssueDescriptor {
    pub severity: &'static str, // "CRITICAL", "WARNING", "INFO"
    pub description: String,
    pub auto_fix_type: Option<&'static str>,
}

/// A pluggable diagnostic rule for static code audits.
pub trait DiagnosticRule: Send + Sync {
    fn name(&self) -> &'static str;

    /// Returns a descriptor if a line violates this rule.
    fn check_line(&self, line: &str, file_path: &Path) -> Option<SafetyIssueDescriptor>;
}

// ----------------- Rule Implementations -----------------

/// Rule: Enforces no_std namespace usage by banning standard library imports.
pub struct NoStdNamespaceRule;
impl DiagnosticRule for NoStdNamespaceRule {
    fn name(&self) -> &'static str {
        "NoStdNamespace"
    }

    fn check_line(&self, line: &str, file_path: &Path) -> Option<SafetyIssueDescriptor> {
        let path_str = file_path.to_string_lossy();
        if path_str.contains("xtask") || path_str.contains("tests") {
            return None;
        }

        if line.contains("use std::")
            && !line.contains("use std::env")
            && !line.contains("use std::process")
        {
            Some(SafetyIssueDescriptor {
                severity: "CRITICAL",
                description: "Illegal reference to standard library namespace 'std::'. Bare-metal modules must use 'core::' or 'alloc::' instead.".to_string(),
                auto_fix_type: Some("convert_to_core"),
            })
        } else {
            None
        }
    }
}

/// Rule: Banned standard blocking mutexes in kernel space.
pub struct BannedMutexRule;
impl DiagnosticRule for BannedMutexRule {
    fn name(&self) -> &'static str {
        "BannedMutexRule"
    }

    fn check_line(&self, line: &str, file_path: &Path) -> Option<SafetyIssueDescriptor> {
        let path_str = file_path.to_string_lossy();
        if path_str.contains("xtask") {
            return None;
        }

        if line.contains("std::sync::Mutex") {
            Some(SafetyIssueDescriptor {
                severity: "CRITICAL",
                description: "Standard blocking Mutex used. Blocking locks rely on scheduler-backed thread parking. Bare-metal kernels must use spinlocks ('spin::Mutex') instead.".to_string(),
                auto_fix_type: Some("convert_to_spin"),
            })
        } else {
            None
        }
    }
}

/// Rule: Unsynchronized mutable static variable scanner.
pub struct UnsynchronizedStaticRule;
impl DiagnosticRule for UnsynchronizedStaticRule {
    fn name(&self) -> &'static str {
        "UnsynchronizedStatic"
    }

    fn check_line(&self, line: &str, _file_path: &Path) -> Option<SafetyIssueDescriptor> {
        if line.contains("static mut ") && !line.contains("unsafe") {
            Some(SafetyIssueDescriptor {
                severity: "WARNING",
                description: "Unsynchronized static mut variable detected. Bare-metal kernels must use atomic types or synchronized spinlocks to prevent data races.".to_string(),
                auto_fix_type: None,
            })
        } else {
            None
        }
    }
}

/// Helper to load all active diagnostic rules.
pub fn get_active_rules() -> Vec<Box<dyn DiagnosticRule>> {
    vec![
        Box::new(NoStdNamespaceRule),
        Box::new(BannedMutexRule),
        Box::new(UnsynchronizedStaticRule),
    ]
}
