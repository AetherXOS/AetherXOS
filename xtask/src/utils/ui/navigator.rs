use anyhow::Result;
use inquire::Confirm;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use crate::utils::logging;

static TIMELINE: Lazy<Mutex<HashMap<String, Duration>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn record_task_metric(name: &str, duration: Duration) {
    if let Ok(mut map) = TIMELINE.lock() {
        map.insert(name.to_string(), duration);
    }
}

pub fn get_timeline() -> HashMap<String, Duration> {
    TIMELINE.lock().map(|m| m.clone()).unwrap_or_default()
}

pub fn diagnose_failure(error: &str) -> Option<String> {
    if error.contains("rustc") {
        Some("Toolchain missing or corrupted. Try running 'xtask setup --tools'.".to_string())
    } else if error.contains("qemu") {
        Some("QEMU not found or version incompatible. Ensure QEMU is in your PATH.".to_string())
    } else if error.contains("lld") || error.contains("linker") {
        Some("Linker error detected. Ensure 'lld' is installed.".to_string())
    } else {
        None
    }
}

pub fn suggest_next_step(current_workflow: &str, success: bool, error_msg: Option<&str>) -> Result<()> {
    if !success {
        if let Some(err) = error_msg {
            if let Some(fix) = diagnose_failure(err) {
                logging::status("ORACLE", &format!("AI Diagnosis: {}", fix));
            }
        }
        logging::info("NAVIGATOR", "Troubleshooting tip: Check 'artifacts/logs/' for detailed failure traces.", &[]);
        return Ok(());
    }

    match current_workflow {
        "full_iso" => {
            if Confirm::new("Workflow complete. Do you want to launch the ISO in QEMU?").prompt()? {
                crate::engine::controller::UniversalController::dispatch_workflow("debug", &crate::engine::ExecutionContext::from_defaults())?;
            }
        }
        "kernel_dev" => {
            if Confirm::new("Kernel compiled. Do you want to run a safety audit?").prompt()? {
                // ... Dispatch audit
            }
        }
        _ => {}
    }

    Ok(())
}
