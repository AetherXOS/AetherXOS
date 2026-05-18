use super::repairs::apply_repair;
use super::rules::get_active_rules;
use crate::utils::logging;
use anyhow::Result;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub message: String,
    pub error_code: Option<String>,
    pub explanation: String,
    pub fix_options: Vec<FixOption>,
}

#[derive(Debug, Clone)]
pub struct FixOption {
    pub label: &'static str,
    pub command: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone)]
pub struct SafetyIssue {
    pub file: PathBuf,
    pub line_num: usize,
    pub severity: &'static str,
    pub description: String,
    pub code_snippet: String,
    pub auto_fix_suggested: Option<String>,
}

pub struct DiagnosticEngine;

impl DiagnosticEngine {
    /// Discover src directories dynamically from Cargo workspace structure
    pub fn discover_src_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let path = entry.path();
                    if path.join("Cargo.toml").exists() && path.join("src").exists() {
                        let name = path.file_name().unwrap().to_string_lossy();
                        if name != "xtask" {
                            dirs.push(path.join("src"));
                        }
                    }
                }
            }
        }
        if dirs.is_empty() {
            dirs.push(PathBuf::from("src"));
        }
        dirs
    }

    /// Run deep diagnosis on compiler log lines
    pub fn diagnose_logs(log_lines: &[String]) -> DiagnosticResult {
        let full_text = log_lines.join("\n");

        let error_code = if let Some(pos) = full_text.find("error[E") {
            if pos + 12 <= full_text.len() {
                Some(full_text[pos + 5..pos + 10].to_string())
            } else {
                None
            }
        } else {
            None
        };

        if let Some(ref code) = error_code {
            match code.as_str() {
                "E0432" => {
                    return DiagnosticResult {
                        message: "Unresolved Import".to_string(),
                        error_code: Some(code.clone()),
                        explanation: "An import could not be resolved. This usually means a dependency doesn't support '#![no_std]', or you imported 'std' instead of 'core'/'alloc'.".to_string(),
                        fix_options: vec![
                            FixOption {
                                label: "🔍 Run AI Code Doctor Scanner",
                                command: "scan_codebase",
                                description: "Scans active workspace directories for illegal 'std::' references and offers automated repairs.",
                            },
                            FixOption {
                                label: "🧹 Clean cargo cache and rebuild",
                                command: "cargo clean",
                                description: "Cleans target build directories to resolve stale or corrupted dependency caches.",
                            }
                        ]
                    };
                }
                "E0463" => {
                    return DiagnosticResult {
                        message: "Missing Crate (usually 'std')".to_string(),
                        error_code: Some(code.clone()),
                        explanation: "The compiler cannot find a core crate. In bare-metal compilation, this happens when a library does not declare '#![no_std]', causing the compiler to look for the non-existent 'std' crate.".to_string(),
                        fix_options: vec![
                            FixOption {
                                label: "🛠️  Inject '#![no_std]' directive",
                                command: "inject_no_std",
                                description: "Injects missing '#![no_std]' declaration at the top of library entry points.",
                            }
                        ]
                    };
                }
                _ => {}
            }
        }

        if full_text.contains("x86_64-unknown-none") {
            DiagnosticResult {
                message: "Target toolchain missing".to_string(),
                error_code: None,
                explanation: "The cross-compilation target 'x86_64-unknown-none' is not installed in your Rust toolchain.".to_string(),
                fix_options: vec![
                    FixOption {
                        label: "⚡ Install x86_64 bare-metal target via rustup",
                        command: "rustup target add x86_64-unknown-none",
                        description: "Adds the official x86_64 cross-compilation target to the active Rust toolchain.",
                    }
                ]
            }
        } else if full_text.contains("ovmf") || full_text.contains("OVMF") {
            DiagnosticResult {
                message: "OVMF Firmware missing".to_string(),
                error_code: None,
                explanation: "OVMF binaries required for booting UEFI systems are missing."
                    .to_string(),
                fix_options: vec![FixOption {
                    label: "📦 Run local setup downloader",
                    command: "cargo run -p xtask -- setup --tools",
                    description: "Downloads QEMU firmware binaries and UEFI assets locally.",
                }],
            }
        } else {
            DiagnosticResult {
                message: "General Compilation or Linker Fault".to_string(),
                error_code: error_code,
                explanation: "The compilation or linkage cycle failed. This could be due to symbol conflicts, missing imports, or incorrect target parameters.".to_string(),
                fix_options: vec![
                    FixOption {
                        label: "🧹 Run Clean Cargo build cycle",
                        command: "cargo clean",
                        description: "Flushes the build cache completely to bypass stale incremental compile glitches.",
                    },
                    FixOption {
                        label: "🏥 Execute AI Code Doctor Health Scan",
                        command: "scan_codebase",
                        description: "Runs decoupled rules scanning unsafe pointer references or wrong blocking mutexes.",
                    }
                ]
            }
        }
    }

    /// Scan codebase recursively using all registered diagnostic rules
    pub fn scan_workspace() -> Result<Vec<SafetyIssue>> {
        let mut issues = Vec::new();
        let src_dirs = Self::discover_src_dirs();
        let rules = get_active_rules();

        for dir in &src_dirs {
            if !dir.exists() {
                continue;
            }
            for entry in walkdir::WalkDir::new(dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file()
                    && entry.path().extension().map_or(false, |ext| ext == "rs")
                {
                    let file_issues = Self::scan_file(entry.path(), &rules)?;
                    issues.extend(file_issues);
                }
            }
        }

        Ok(issues)
    }

    fn scan_file(
        path: &Path,
        rules: &[Box<dyn crate::utils::ui::oracle::rules::DiagnosticRule>],
    ) -> Result<Vec<SafetyIssue>> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let mut issues = Vec::new();
        let lines: Vec<&str> = contents.lines().collect();

        // 1. Run dynamic rules line by line
        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1;
            for rule in rules {
                if let Some(desc) = rule.check_line(line, path) {
                    issues.push(SafetyIssue {
                        file: path.to_path_buf(),
                        line_num,
                        severity: desc.severity,
                        description: desc.description,
                        code_snippet: line.trim().to_string(),
                        auto_fix_suggested: desc.auto_fix_type.map(|s| s.to_string()),
                    });
                }
            }
        }

        // 2. Extra workspace-wide safety checks (missing no_std in lib entry points)
        let is_lib_entry = path
            .file_name()
            .map_or(false, |n| n == "lib.rs" || n == "main.rs");
        if is_lib_entry
            && !contents.contains("#![no_std]")
            && !path.to_string_lossy().contains("xtask")
        {
            issues.push(SafetyIssue {
                file: path.to_path_buf(),
                line_num: 1,
                severity: "CRITICAL",
                description: "Missing '#![no_std]' directive in library root file. Bare-metal modules must compile without standard library ties.".to_string(),
                code_snippet: lines.first().cloned().unwrap_or("").to_string(),
                auto_fix_suggested: Some("inject_no_std".to_string()),
            });
        }

        Ok(issues)
    }

    /// Applies auto-repair strategies to physical files
    pub fn repair_issue(issue: &SafetyIssue) -> Result<()> {
        let mut file = File::open(&issue.file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        if let Some(ref fix_type) = issue.auto_fix_suggested {
            if let Some(new_contents) = apply_repair(&contents, fix_type) {
                let mut out = File::create(&issue.file)?;
                out.write_all(new_contents.as_bytes())?;
                logging::ready(
                    "AI_REPAIR",
                    &format!("Successfully applied repair '{}'!", fix_type),
                    "ok",
                    &[],
                );
            }
        }

        Ok(())
    }
}
