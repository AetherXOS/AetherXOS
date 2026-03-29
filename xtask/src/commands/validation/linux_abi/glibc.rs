use anyhow::Result;
use std::collections::HashMap;
use std::fs;

use crate::utils::{paths, report};

pub fn run_analysis() -> Result<()> {
    println!("[linux-abi::glibc] Analyzing syscall stubs blocking glibc compatibility");

    let root = paths::repo_root();
    let gap_report_path = root.join("reports/abi_gap_inventory/summary.json");
    if !gap_report_path.exists() {
        println!("[linux-abi::glibc] Gap report not found, running gap inventory first");
        crate::commands::validation::linux_abi::gap::run()?;
    }

    let gap_data: serde_json::Value = serde_json::from_str(&fs::read_to_string(&gap_report_path)?)?;
    let entries = gap_data.get("entries").and_then(|e| e.as_array()).unwrap();

    let priorities = create_priorities();

    let mut analysis: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    let mut total_critical = 0;

    for entry in entries {
        if entry.get("category").and_then(|c| c.as_str()) != Some("stub") {
            continue;
        }

        let fn_name = entry.get("function").and_then(|f| f.as_str()).unwrap();
        let syscall_name = fn_name.strip_prefix("sys_linux_").unwrap_or(fn_name);

        let priority = priorities.get(syscall_name).cloned().unwrap_or_else(|| "OTHER".to_string());
        if priority.starts_with("CRITICAL") {
            total_critical += 1;
        }

        analysis.entry(priority).or_default().push(entry.clone());
    }

    let out_dir = paths::resolve("reports/glibc_needs");
    paths::ensure_dir(&out_dir)?;

    report::write_json_report(&out_dir.join("summary.json"), &serde_json::json!({
        "total_critical_stubs": total_critical,
        "breakdown": analysis,
    }))?;

    println!("{}", "=".repeat(60));
    println!("GLIBC COMPATIBILITY BLOCKER ANALYSIS");
    println!("{}", "=".repeat(60));
    println!();

    let display_order = vec![
        "CRITICAL_FILE_IO", "CRITICAL_PROCESS", "CRITICAL_MEMORY", "CRITICAL_SIGNALS",
        "IMPORTANT_THREADING", "IMPORTANT_FD_OPS", "IMPORTANT_FS", "SUPPORT", "OTHER"
    ];

    for category in display_order {
        if let Some(category_stubs) = analysis.get(category) {
            println!("\n{category}: Stub Count {}", category_stubs.len());
            println!("{}", "-".repeat(60));
            for stub in category_stubs {
                let f_name = stub.get("function").and_then(|f| f.as_str()).unwrap();
                let file_path = stub.get("file").and_then(|f| f.as_str()).unwrap();
                let file_name = file_path.rsplit('/').next().unwrap();
                println!("  \u{2022} {f_name:30} in {file_name}");
            }
        }
    }

    println!("\nTOTAL CRITICAL STUBS BLOCKING GLIBC: {total_critical}");
    println!("{}", "=".repeat(60));

    Ok(())
}

fn create_priorities() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let categories: Vec<(&str, &[&str])> = vec![
        ("CRITICAL_FILE_IO", &["read", "write", "open", "openat", "close", "lseek", "getdents64", "readdir", "readv", "writev", "preadv", "pwritev"]),
        ("CRITICAL_PROCESS", &["fork", "clone", "clone3", "execve", "execveat", "wait4", "waitpid", "exit", "exit_group"]),
        ("CRITICAL_MEMORY", &["mmap", "mmap2", "munmap", "brk", "mprotect", "mremap"]),
        ("CRITICAL_SIGNALS", &["rt_sigaction", "rt_sigprocmask", "rt_sigpending", "rt_sigtimedwait", "sigaltstack"]),
        ("IMPORTANT_THREADING", &["futex", "futex2", "clone3", "set_tid_address", "set_robust_list", "get_robust_list"]),
        ("IMPORTANT_FD_OPS", &["dup", "dup2", "dup3", "pipe", "pipe2", "poll", "epoll_create", "epoll_wait", "select"]),
        ("IMPORTANT_FS", &["stat", "lstat", "fstat", "statx", "access", "faccessat", "mkdir", "rmdir", "rename", "unlink"]),
        ("SUPPORT", &["time", "clock_gettime", "gettimeofday", "utime", "utimens"]),
    ];

    for (cat, calls) in categories {
        for call in calls {
            map.insert(call.to_string(), cat.to_string());
        }
    }
    map
}
