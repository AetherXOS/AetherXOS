pub mod rules;
pub mod repairs;
pub mod engine;

use anyhow::Result;
use inquire::{Confirm, Select};
use colored::Colorize;
use crate::engine::ExecutionContext;
use std::fs::File;
use std::io::Read;

pub struct Oracle;

impl Oracle {
    /// Suggest the logical next step or offer auto-fix suggestions.
    /// Returns Ok(true) if an auto-fix was applied successfully and the user wishes to retry.
    pub fn suggest_next(workflow: &str, success: bool, error_msg: Option<&str>) -> Result<bool> {
        if !success {
            let mut fix_applied = false;
            
            // Gather last 30 lines from logs to build high-fidelity diagnostics
            let mut log_lines = Vec::new();
            if let Some(err) = error_msg {
                log_lines.push(err.to_string());
            }
            if let Ok(mut file) = File::open("artifacts/logs/xtask.log") {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    let lines: Vec<String> = contents.lines().map(|s| s.to_string()).collect();
                    let start = lines.len().saturating_sub(30);
                    log_lines.extend_from_slice(&lines[start..]);
                }
            }

            let diag = engine::DiagnosticEngine::diagnose_logs(&log_lines);
            
            println!("\n🧠  AI Diagnostics: {}", diag.message.bold().cyan());
            println!("   Explanation: {}\n", diag.explanation.white());

            if !diag.fix_options.is_empty() {
                let mut options: Vec<String> = diag.fix_options.iter().map(|f| f.label.to_string()).collect();
                options.push("❌ Cancel and view raw log trace".to_string());

                let selection = Select::new("🔧 Select Auto-Repair Strategy:", options).prompt()?;
                
                if let Some(fix) = diag.fix_options.iter().find(|f| f.label == selection) {
                    println!("\nApplying AI Strategy: {}...", fix.label.bold().yellow());
                    println!("Description: {}\n", fix.description);

                    if Confirm::new("Execute this fix command now?").prompt().unwrap_or(false) {
                        match fix.command {
                            "scan_codebase" => {
                                println!("Scanning workspace code health...");
                                let issues = engine::DiagnosticEngine::scan_workspace()?;
                                if issues.is_empty() {
                                    println!("🟢 AI Code Doctor found 0 safety issues.");
                                } else {
                                    println!("\n🏥 AI Code Doctor scanned and found {} issues:\n", issues.len());
                                    for (i, issue) in issues.iter().enumerate() {
                                        println!("  [{}] [{}] {}:{}", i + 1, issue.severity.red(), issue.file.display(), issue.line_num);
                                        println!("       Description: {}", issue.description);
                                        println!("       Code: '{}'", issue.code_snippet.trim().yellow());
                                        if issue.auto_fix_suggested.is_some() {
                                            if Confirm::new("   Apply auto-fix repair for this issue?").prompt().unwrap_or(false) {
                                                engine::DiagnosticEngine::repair_issue(issue)?;
                                                fix_applied = true;
                                            }
                                        }
                                        println!();
                                    }
                                }
                            }
                            "inject_no_std" => {
                                let issues = engine::DiagnosticEngine::scan_workspace()?;
                                if let Some(entry_issue) = issues.iter().find(|i| i.auto_fix_suggested.as_deref() == Some("inject_no_std")) {
                                    engine::DiagnosticEngine::repair_issue(entry_issue)?;
                                    fix_applied = true;
                                } else {
                                    println!("Could not find missing no_std entry points.");
                                }
                            }
                            raw_cmd => {
                                let parts: Vec<&str> = raw_cmd.split_whitespace().collect();
                                if !parts.is_empty() {
                                    let mut runner = std::process::Command::new(parts[0]);
                                    if parts.len() > 1 {
                                        runner.args(&parts[1..]);
                                    }
                                    match runner.status() {
                                        Ok(status) if status.success() => {
                                            println!("🎉 Strategy applied successfully!");
                                            fix_applied = true;
                                        }
                                        _ => {
                                            println!("❌ Failed to execute fix command.");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if fix_applied {
                if Confirm::new("Auto-fix applied. Would you like to RETRY the build now?").prompt().unwrap_or(false) {
                    return Ok(true);
                }
            }
            return Ok(false);
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

        Ok(false)
    }

    fn dispatch(workflow: &str) -> Result<()> {
        crate::engine::controller::UniversalController::dispatch_workflow(
            workflow, 
            &ExecutionContext::from_defaults()
        )
    }
}
