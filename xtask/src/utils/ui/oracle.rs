use anyhow::Result;
use inquire::Confirm;
use crate::utils::logging;
use crate::engine::ExecutionContext;

/// The Oracle provides intelligent build diagnostics and navigation suggestions.
pub struct Oracle;

impl Oracle {
    /// Diagnose a build failure based on known error patterns.
    pub fn diagnose(error: &str) -> Option<&'static str> {
        const PATTERNS: &[(&str, &str)] = &[
            ("rustc", "Toolchain missing or corrupted. Try: 'xtask setup --tools'"),
            ("qemu", "QEMU not found or version incompatible. Check your PATH."),
            ("lld", "Linker error: 'lld' missing. Required for bare-metal builds."),
            ("linker", "Linker error: Ensure cross-compilation targets are installed."),
            ("ovmf", "OVMF firmware missing. Required for UEFI Secure Boot testing."),
        ];

        for (pattern, fix) in PATTERNS {
            if error.contains(pattern) {
                return Some(fix);
            }
        }
        None
    }

    /// Suggest the logical next step in the development workflow.
    pub fn suggest_next(workflow: &str, success: bool, error_msg: Option<&str>) -> Result<()> {
        if !success {
            if let Some(err) = error_msg {
                if let Some(fix) = Self::diagnose(err) {
                    logging::status("ORACLE", &format!("AI Diagnosis: {}", fix));
                }
            }
            logging::info("ORACLE", "Tip: Check 'artifacts/logs/xtask.log' for full traces.", &[]);
            return Ok(());
        }

        match workflow {
            "full_iso"
                if Confirm::new("ISO Build Complete. Launch in QEMU?").prompt()? => {
                    Self::dispatch("debug")?;
                }
            "kernel"
                if Confirm::new("Kernel Ready. Run Safety Audit?").prompt()? => {
                    Self::dispatch("audit")?;
                }
            _ => {}
        }

        Ok(())
    }

    fn dispatch(workflow: &str) -> Result<()> {
        crate::engine::controller::UniversalController::dispatch_workflow(
            workflow, 
            &ExecutionContext::from_defaults()
        )
    }
}
