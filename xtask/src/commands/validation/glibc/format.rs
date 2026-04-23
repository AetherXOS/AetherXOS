use super::{ClosureTestResult, GlibcSyscall, SyscallStatus};
use anyhow::Result;
use std::collections::HashMap;

pub fn generate_markdown(inventory: &[GlibcSyscall], _verbose: bool) -> Result<String> {
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

    let full_rate = if total > 0 { full as f64 / total as f64 * 100.0 } else { 0.0 };
    let partial_rate = if total > 0 { partial as f64 / total as f64 * 100.0 } else { 0.0 };
    let stub_rate = if total > 0 { stub as f64 / total as f64 * 100.0 } else { 0.0 };

    md.push_str(&format!(
        "\n## Summary\n\n- **Total:** {} syscalls\n- **Full:** {} ({:.1}%)\n- **Partial:** {} ({:.1}%)\n- **Stub:** {} ({:.1}%)\n",
        total, full, full_rate, partial, partial_rate, stub, stub_rate
    ));

    Ok(md)
}

pub fn generate_csv(inventory: &[GlibcSyscall], _verbose: bool) -> Result<String> {
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

pub fn generate_closure_markdown(results: &[ClosureTestResult]) -> Result<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_csv_empty() {
        let inventory = vec![];
        let csv = generate_csv(&inventory, false).unwrap();
        assert_eq!(csv, "Syscall,Family,Status,Location,Issues,Tests\n");
    }

    #[test]
    fn test_generate_markdown_empty() {
        let inventory = vec![];
        let md = generate_markdown(&inventory, false).unwrap();
        assert!(md.contains("# glibc Syscall Audit Report"));
        assert!(md.contains("- **Total:** 0 syscalls"));
    }
}
