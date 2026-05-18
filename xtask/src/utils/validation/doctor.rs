use crate::config;
use crate::utils::fs::paths::LAYOUT;
use crate::utils::{logging, process, report};
use anyhow::{Result, bail};
use serde::Serialize;
use std::process::Command;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

/// Represents a single diagnostic check in the Nexus Doctor system.
pub trait Diagnostic: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, strict: bool) -> Result<DiagnosticResult>;
}

pub enum DiagnosticResult {
    Pass(String),
    Warn(String, String), // Message, Recommendation
    Fail(String, String), // Message, Recommendation
}

/// A structured requirement for the build environment.
struct Requirement {
    cmd: &'static str,
    name: &'static str,
    critical: bool,
    recommendation: &'static str,
}

impl Requirement {
    fn check(&self, strict: bool) -> DiagnosticResult {
        if process::which(self.cmd) {
            DiagnosticResult::Pass(format!("{} verified", self.name))
        } else {
            let msg = format!(
                "Missing {}: {}",
                if self.critical {
                    "critical tool"
                } else {
                    "optional tool"
                },
                self.name
            );
            let rec = self.recommendation.to_string();
            if self.critical && strict {
                DiagnosticResult::Fail(msg, rec)
            } else {
                DiagnosticResult::Warn(msg, rec)
            }
        }
    }
}

pub struct NexusDoctor {
    diagnostics: Vec<Box<dyn Diagnostic>>,
}

impl NexusDoctor {
    pub fn new() -> Self {
        Self {
            diagnostics: vec![
                Box::new(HardwareDiagnostic),
                Box::new(StorageDiagnostic),
                Box::new(ToolchainDiagnostic),
                Box::new(BinaryDiagnostic),
                Box::new(EnvironmentDiagnostic),
            ],
        }
    }

    pub fn audit(&self, strict: bool) -> Result<()> {
        logging::status("DOCTOR", "Initiating system-wide health audit...");
        let mut has_errors = false;

        for diagnostic in &self.diagnostics {
            match diagnostic.run(strict) {
                Ok(DiagnosticResult::Pass(msg)) => {
                    logging::success("DOCTOR", &format!("{}: {}", diagnostic.name(), msg), &[]);
                }
                Ok(DiagnosticResult::Warn(msg, rec)) => {
                    logging::warn(
                        "DOCTOR",
                        &format!("{}: {}", diagnostic.name(), msg),
                        &[("action", &rec)],
                    );
                }
                Ok(DiagnosticResult::Fail(msg, rec)) => {
                    logging::error(
                        "DOCTOR",
                        &format!("{}: {}", diagnostic.name(), msg),
                        &[("action", &rec)],
                    );
                    has_errors = true;
                }
                Err(e) => {
                    logging::error(
                        "DOCTOR",
                        &format!("Diagnostic failure in {}: {}", diagnostic.name(), e),
                        &[],
                    );
                    has_errors = true;
                }
            }
        }

        if has_errors && strict {
            bail!("Nexus Doctor: Critical system health issues detected. Execution halted.");
        }

        Ok(())
    }
}

// --- Specific Diagnostics ---

struct HardwareDiagnostic;
impl Diagnostic for HardwareDiagnostic {
    fn name(&self) -> &str {
        "Hardware"
    }
    fn run(&self, _strict: bool) -> Result<DiagnosticResult> {
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        sys.refresh_all();

        let ram_gb = sys.total_memory() / 1024 / 1024 / 1024;
        let cores = sys.cpus().len();

        if ram_gb < 8 {
            return Ok(DiagnosticResult::Warn(
                format!("RAM: {}GB", ram_gb),
                "Recommended: 8GB+".to_string(),
            ));
        }

        Ok(DiagnosticResult::Pass(format!(
            "{}GB RAM, {} Cores",
            ram_gb, cores
        )))
    }
}

struct StorageDiagnostic;
impl Diagnostic for StorageDiagnostic {
    fn name(&self) -> &str {
        "Storage"
    }
    fn run(&self, strict: bool) -> Result<DiagnosticResult> {
        let disks = Disks::new_with_refreshed_list();
        let mut available_gb = 0;

        for disk in &disks {
            if LAYOUT.root.starts_with(disk.mount_point()) {
                available_gb = disk.available_space() / 1024 / 1024 / 1024;
                break;
            }
        }

        if available_gb < 15 {
            let msg = format!("Low disk space: {}GB", available_gb);
            let rec = "Requirement: 15GB+ available in project volume".to_string();
            return Ok(if strict {
                DiagnosticResult::Fail(msg, rec)
            } else {
                DiagnosticResult::Warn(msg, rec)
            });
        }

        Ok(DiagnosticResult::Pass(format!(
            "{}GB available",
            available_gb
        )))
    }
}

struct ToolchainDiagnostic;
impl Diagnostic for ToolchainDiagnostic {
    fn name(&self) -> &str {
        "Toolchain"
    }
    fn run(&self, strict: bool) -> Result<DiagnosticResult> {
        let output = Command::new("rustc").arg("-V").output();
        if let Ok(out) = output {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();

            // Validate target x86_64-unknown-none
            let target_check = Command::new("rustup")
                .args(["target", "list", "--installed"])
                .output();
            if let Ok(t_out) = target_check {
                let list = String::from_utf8_lossy(&t_out.stdout);
                if !list.contains("x86_64-unknown-none") {
                    let msg = "Missing target: x86_64-unknown-none".to_string();
                    let rec = "Run: rustup target add x86_64-unknown-none".to_string();
                    return Ok(if strict {
                        DiagnosticResult::Fail(msg, rec)
                    } else {
                        DiagnosticResult::Warn(msg, rec)
                    });
                }
            }

            Ok(DiagnosticResult::Pass(version))
        } else {
            Ok(DiagnosticResult::Fail(
                "Rust toolchain not found".to_string(),
                "Install via https://rustup.rs".to_string(),
            ))
        }
    }
}

struct BinaryDiagnostic;
impl Diagnostic for BinaryDiagnostic {
    fn name(&self) -> &str {
        "Binaries"
    }
    fn run(&self, strict: bool) -> Result<DiagnosticResult> {
        let requirements = vec![
            Requirement {
                cmd: "git",
                name: "Git VCS",
                critical: true,
                recommendation: "Install Git",
            },
            Requirement {
                cmd: "curl",
                name: "Curl",
                critical: true,
                recommendation: "Install curl for external asset fetching",
            },
            Requirement {
                cmd: "qemu-system-x86_64",
                name: "QEMU (x86_64)",
                critical: false,
                recommendation: "Install QEMU for emulation",
            },
            Requirement {
                cmd: "7z",
                name: "7-Zip / p7zip",
                critical: false,
                recommendation: "Install 7-Zip for ISO content inspection",
            },
            Requirement {
                cmd: "xorriso",
                name: "Xorriso",
                critical: false,
                recommendation: "Install xorriso for ISO assembly",
            },
        ];

        let mut warnings = Vec::new();
        for req in requirements {
            match req.check(strict) {
                DiagnosticResult::Fail(m, r) => return Ok(DiagnosticResult::Fail(m, r)),
                DiagnosticResult::Warn(m, _) => warnings.push(m),
                _ => {}
            }
        }

        if !warnings.is_empty() {
            Ok(DiagnosticResult::Warn(
                format!("{} optional tools missing", warnings.len()),
                "Consider installing missing tools for full feature support".to_string(),
            ))
        } else {
            Ok(DiagnosticResult::Pass(
                "All essential and optional binaries verified".to_string(),
            ))
        }
    }
}

struct EnvironmentDiagnostic;
impl Diagnostic for EnvironmentDiagnostic {
    fn name(&self) -> &str {
        "Environment"
    }
    fn run(&self, _strict: bool) -> Result<DiagnosticResult> {
        // Check for workspace stability
        if !LAYOUT.root.exists() {
            return Ok(DiagnosticResult::Fail(
                "Project root invalid".to_string(),
                "Ensure xtask is run from project root".to_string(),
            ));
        }

        // Check for artifacts directory
        if !LAYOUT.artifacts.exists() {
            return Ok(DiagnosticResult::Warn(
                "Artifacts directory missing".to_string(),
                "Will be created automatically during build".to_string(),
            ));
        }

        Ok(DiagnosticResult::Pass(
            "Workspace environment stable".to_string(),
        ))
    }
}

// --- Preflight Host Tool Verification ---

#[derive(Serialize)]
pub struct HostToolCheck {
    pub id: String,
    pub required: bool,
    pub found: bool,
    pub detail: String,
}

#[derive(Serialize)]
pub struct HostToolVerifyReport {
    pub generated_utc: String,
    pub strict: bool,
    pub overall_ok: bool,
    pub required_missing: usize,
    pub checks: Vec<HostToolCheck>,
}

pub fn host_tool_verify_report(strict: bool) -> Result<()> {
    println!("[release::host-tool-verify] Generating NexusDoctor preflight host tool report");

    // We can reuse the requirements from BinaryDiagnostic and ToolchainDiagnostic
    let requirements = vec![
        Requirement {
            cmd: "rustc",
            name: "Rust Compiler",
            critical: true,
            recommendation: "Install Rust",
        },
        Requirement {
            cmd: "cargo",
            name: "Cargo",
            critical: true,
            recommendation: "Install Rust",
        },
        Requirement {
            cmd: "git",
            name: "Git VCS",
            critical: true,
            recommendation: "Install Git",
        },
        Requirement {
            cmd: "curl",
            name: "Curl",
            critical: true,
            recommendation: "Install curl",
        },
        Requirement {
            cmd: "qemu-system-x86_64",
            name: "QEMU (x86_64)",
            critical: false,
            recommendation: "Install QEMU",
        },
        Requirement {
            cmd: "7z",
            name: "7-Zip / p7zip",
            critical: false,
            recommendation: "Install 7-Zip",
        },
        Requirement {
            cmd: "xorriso",
            name: "Xorriso",
            critical: false,
            recommendation: "Install xorriso",
        },
        Requirement {
            cmd: "python",
            name: "Python",
            critical: false,
            recommendation: "Install Python for reporting",
        },
    ];

    let mut checks = Vec::with_capacity(requirements.len());
    let mut required_missing = 0;

    for req in requirements {
        let found = process::which(req.cmd);
        if req.critical && !found {
            required_missing += 1;
        }

        let detail = if found {
            format!("{} verified", req.name)
        } else {
            format!(
                "Missing {}. Recommendation: {}",
                req.name, req.recommendation
            )
        };

        checks.push(HostToolCheck {
            id: req.cmd.to_string(),
            required: req.critical,
            found,
            detail,
        });
    }

    let overall_ok = required_missing == 0;

    let report_obj = HostToolVerifyReport {
        generated_utc: report::utc_now_iso(),
        strict,
        overall_ok,
        required_missing,
        checks,
    };

    let root = &LAYOUT.root;
    let out_json = root.join(config::repo_paths::HOST_TOOL_VERIFY_JSON);
    let out_md = root.join(config::repo_paths::HOST_TOOL_VERIFY_MD);

    report::write_json_report(&out_json, &report_obj)?;
    report::write_text_report(&out_md, &render_host_tool_verify_md(&report_obj))?;

    if strict && !report_obj.overall_ok {
        bail!(
            "strict host tool verify failed: required_missing={}. See {}",
            report_obj.required_missing,
            out_json.display()
        );
    }

    println!("[release::host-tool-verify] PASS");
    Ok(())
}

fn render_host_tool_verify_md(report_obj: &HostToolVerifyReport) -> String {
    let mut md = String::new();
    md.push_str("# Host Tool Verify\n\n");
    md.push_str(&format!("- generated_utc: {}\n", report_obj.generated_utc));
    md.push_str(&format!("- strict: {}\n", report_obj.strict));
    md.push_str(&format!("- overall_ok: {}\n", report_obj.overall_ok));
    md.push_str(&format!(
        "- required_missing: {}\n\n",
        report_obj.required_missing
    ));
    md.push_str("## Checks\n\n");
    for check in &report_obj.checks {
        md.push_str(&format!(
            "- [{}] {} (required={})\n",
            if check.found { "x" } else { " " },
            check.id,
            check.required
        ));
        md.push_str(&format!("  - detail: {}\n", check.detail));
    }
    md
}
