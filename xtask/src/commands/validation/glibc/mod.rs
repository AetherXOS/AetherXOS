// glibc syscall completeness audit, closure testing, and tracking
//
// This module provides comprehensive tracking of AetherCore's glibc compatibility progress.
// Currently at 47/50 critical syscalls fully implemented (94% coverage).
// Remaining blockers: remap_file_pages (deprecated), clone namespaces, statx extended attrs.

use crate::cli::GlibcAction;
use crate::commands::validation::reports::glibc as glibc_reports;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

/// Represents implementation status of a syscall
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum SyscallStatus {
    #[serde(rename = "FULL")]
    Full,
    #[serde(rename = "PARTIAL")]
    Partial,
    #[serde(rename = "STUB")]
    Stub,
}

/// Comprehensive glibc syscall inventory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlibcSyscall {
    pub name: String,
    pub family: String,
    pub status: SyscallStatus,
    pub location: String,
    pub issues: Vec<String>,
    pub tests: Vec<String>,
    pub dependencies: Vec<String>,
}

mod inventory;
pub(crate) use inventory::{SyscallDef, get_glibc_inventory};

pub fn execute(action: &GlibcAction) -> Result<()> {
    match action {
        GlibcAction::Audit {
            format,
            out,
            verbose,
        } => {
            execute_audit(format, out, *verbose)?;
        }
        GlibcAction::ClosureGate {
            quick,
            strict,
            family,
            format,
            out,
        } => {
            execute_closure_gate(*quick, *strict, family.as_deref(), format, out)?;
        }
        GlibcAction::Scorecard { format, out } => glibc_reports::execute_scorecard(format, out)?,
        GlibcAction::CompatibilitySplit { strict } => {
            glibc_reports::execute_compatibility_split(*strict)?;
        }
    };
    Ok(())
}

fn execute_audit(format: &str, out: &Option<String>, verbose: bool) -> Result<()> {
    let inventory_defs = get_glibc_inventory();
    let inventory: Vec<GlibcSyscall> = inventory_defs
        .iter()
        .map(|d| GlibcSyscall {
            name: d.name.to_string(),
            family: d.family.to_string(),
            status: d.status,
            location: d.location.to_string(),
            issues: d.issues.iter().map(|s| s.to_string()).collect(),
            tests: d.tests.iter().map(|s| s.to_string()).collect(),
            dependencies: d.dependencies.iter().map(|s| s.to_string()).collect(),
        })
        .collect();

    let output = match format {
        "json" => serde_json::to_string_pretty(&inventory)?,
        "csv" => generate_csv(&inventory, verbose)?,
        _ => generate_markdown(&inventory, verbose)?,
    };

    if let Some(path) = out {
        fs::write(path, &output)?;
        eprintln!("✓ Glibc audit written to {}", path);
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn execute_closure_gate(
    quick: bool,
    strict: bool,
    family: Option<&str>,
    format: &str,
    out: &Option<String>,
) -> Result<()> {
    // Determine which syscall families to test
    let families_to_test: Vec<&str> = if let Some(fam) = family {
        let valid_families = ["file_io", "process", "memory", "signals", "threading"];
        if valid_families.contains(&fam) {
            vec![fam]
        } else {
            bail!(
                "Unknown family '{}'. Valid families: file_io, process, memory, signals, threading",
                fam
            );
        }
    } else if quick {
        vec!["file_io", "process", "memory"]
    } else {
        vec!["file_io", "process", "memory", "signals", "threading"]
    };

    let mut results = Vec::new();

    for fam in families_to_test {
        let (passed, failed, blockers) = test_family(fam, strict)?;
        results.push(ClosureTestResult {
            family: fam.to_string(),
            passed,
            failed,
            blockers,
        });
    }

    let output = match format {
        "json" => serde_json::to_string_pretty(&results)?,
        _ => generate_closure_markdown(&results)?,
    };

    if let Some(path) = out {
        fs::write(path, &output)?;
        eprintln!("✓ Closure gate results written to {}", path);
    } else {
        println!("{}", output);
    }

    if strict && results.iter().any(|r| r.failed > 0) {
        let total_failed: usize = results.iter().map(|r| r.failed).sum();
        bail!(
            "Closure gate failed in strict mode: {} tests failed",
            total_failed
        );
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct ClosureTestResult {
    family: String,
    passed: usize,
    failed: usize,
    blockers: Vec<String>,
}

fn test_family(family: &str, strict: bool) -> Result<(usize, usize, Vec<String>)> {
    let inventory = get_glibc_inventory();
    let syscalls: Vec<_> = inventory.iter().filter(|d| d.family == family).collect();

    let mut passed = 0;
    let mut failed = 0;
    let mut blockers = Vec::new();

    for syscall in syscalls {
        if syscall.status == SyscallStatus::Stub {
            failed += 1;
            blockers.push(format!("{}: STUB (not implemented)", syscall.name));
            continue;
        }

        if syscall.status == SyscallStatus::Partial {
            if strict {
                failed += 1;
                for issue in syscall.issues {
                    blockers.push(format!("{}: {}", syscall.name, issue));
                }
            } else {
                passed += 1;
            }
            continue;
        }

        if strict && !syscall.issues.is_empty() {
            failed += 1;
            for issue in syscall.issues {
                blockers.push(format!("{}: {}", syscall.name, issue));
            }
        } else {
            passed += 1;
        }
    }

    Ok((passed, failed, blockers))
}

fn generate_markdown(inventory: &[GlibcSyscall], _verbose: bool) -> Result<String> {
    let mut md = String::new();
    md.push_str("# glibc Syscall Audit Report\n\n");

    let mut by_family: HashMap<&str, Vec<_>> = HashMap::new();
    for item in inventory {
        by_family
            .entry(item.family.as_str())
            .or_insert_with(Vec::new)
            .push(item);
    }

    for family in &["file_io", "process", "memory", "signals", "threading"] {
        if let Some(syscalls) = by_family.get(*family) {
            md.push_str(&format!(
                "## {}\n\n",
                family.replace('_', " ").to_uppercase()
            ));
            md.push_str("| Syscall | Status | Location | Issues | Tests |\n");
            md.push_str("|---------|--------|----------|--------|-------|\n");

            for s in syscalls {
                let issues_str = if s.issues.is_empty() {
                    "—".to_string()
                } else {
                    s.issues.join("; ")
                };
                let tests_str = s.tests.join(", ");
                md.push_str(&format!(
                    "| {} | {:?} | {} | {} | {} |\n",
                    s.name, s.status, s.location, issues_str, tests_str
                ));
            }
            md.push('\n');
        }
    }

    let total = inventory.len();
    let full = inventory
        .iter()
        .filter(|s| s.status == SyscallStatus::Full)
        .count();
    let partial = inventory
        .iter()
        .filter(|s| s.status == SyscallStatus::Partial)
        .count();
    let stub = inventory
        .iter()
        .filter(|s| s.status == SyscallStatus::Stub)
        .count();

    md.push_str(&format!(
        "\n## Summary\n\n- **Total:** {} syscalls\n- **Full:** {} ({:.1}%)\n- **Partial:** {} ({:.1}%)\n- **Stub:** {} ({:.1}%)\n",
        total, full, (full as f64 / total as f64 * 100.0), partial, (partial as f64 / total as f64 * 100.0), stub, (stub as f64 / total as f64 * 100.0)
    ));

    Ok(md)
}

fn generate_csv(inventory: &[GlibcSyscall], _verbose: bool) -> Result<String> {
    let mut csv = String::from("Syscall,Family,Status,Location,Issues,Tests\n");

    for s in inventory {
        let issues_str = s.issues.join("; ");
        let tests_str = s.tests.join("; ");
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{:?}\",\"{}\",\"{}\",\"{}\"\n",
            s.name, s.family, s.status, s.location, issues_str, tests_str
        ));
    }

    Ok(csv)
}

fn generate_closure_markdown(results: &[ClosureTestResult]) -> Result<String> {
    let mut md = String::new();
    md.push_str("# glibc Closure Gate Report\n\n");

    let mut total_passed = 0;
    let mut total_failed = 0;

    for result in results {
        let pass_rate = if result.passed + result.failed > 0 {
            (result.passed as f64 / (result.passed + result.failed) as f64 * 100.0) as u32
        } else {
            0
        };

        md.push_str(&format!(
            "## {}\n- **Passed:** {}\n- **Failed:** {}\n- **Rate:** {}%\n",
            result.family.replace('_', " "),
            result.passed,
            result.failed,
            pass_rate
        ));

        if !result.blockers.is_empty() {
            md.push_str("\n### Blockers\n");
            for blocker in &result.blockers {
                md.push_str(&format!("- {}\n", blocker));
            }
        }
        md.push('\n');

        total_passed += result.passed;
        total_failed += result.failed;
    }

    let overall_rate = if total_passed + total_failed > 0 {
        (total_passed as f64 / (total_passed + total_failed) as f64 * 100.0) as u32
    } else {
        0
    };

    md.push_str(&format!(
        "\n## Overall\n- **Passed:** {}\n- **Failed:** {}\n- **Pass Rate:** {}%\n",
        total_passed, total_failed, overall_rate
    ));

    Ok(md)
}
