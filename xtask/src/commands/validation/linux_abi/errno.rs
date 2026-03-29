use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;

use crate::utils::{paths, report};
use crate::commands::validation::linux_abi::utils::{extract_fn_body, normalize_whitespace};

#[derive(Serialize)]
pub struct ErrnoCheckResult {
    pub function: String,
    pub requirement: String,
    pub ok: bool,
    pub file: String,
    pub detail: String,
}

pub fn run_conformance() -> Result<()> {
    println!("[linux-abi::errno] Running static errno conformance checks");

    let root = paths::repo_root();
    let target_file = "src/modules/linux_compat/net/poll.rs";
    let target = root.join(target_file);
    let out_dir = paths::resolve("reports/errno_conformance");
    paths::ensure_dir(&out_dir)?;

    let text = fs::read_to_string(&target)
        .with_context(|| format!("Failed to read: {}", target.display()))?;

    let checks: Vec<(&str, &str, &[&str])> = vec![
        ("sys_linux_epoll_create", "reject zero size", &["linux_inval()"]),
        ("sys_linux_epoll_pwait", "reject zero/maxevents overflow",
            &["maxevents == 0 || maxevents > MAX_EPOLL_EVENTS"]),
        ("sys_linux_ppoll", "reject poll fd overflow", &["nfds > MAX_POLL_FDS"]),
        ("sys_linux_pselect6", "reject fdset overflow", &["nfds > LINUX_FD_SETSIZE"]),
    ];

    let mut results = Vec::new();
    for (func, req, tokens) in &checks {
        let body = extract_fn_body(&text, func);
        let body_norm = normalize_whitespace(&body);
        let ok = tokens.iter().any(|t| body_norm.contains(t));
        results.push(ErrnoCheckResult {
            function: func.to_string(),
            requirement: req.to_string(),
            ok,
            file: target_file.to_string(),
            detail: if ok { "matched".to_string() } else { format!("missing tokens: {}", tokens.join(", ")) },
        });
    }

    let passed = results.iter().filter(|r| r.ok).count();
    let total = results.len();

    let payload = serde_json::json!({
        "summary": { "ok": passed == total, "checks": total, "passed": passed, "failed": total - passed },
        "results": results,
    });

    report::write_json_report(&out_dir.join("summary.json"), &payload)?;
    println!("[linux-abi::errno] {}/{} checks passed", passed, total);
    Ok(())
}

pub fn run_shim_conformance() -> Result<()> {
    println!("[linux-abi::shim-errno] Checking shim errno conformance (EFAULT mapping)");

    let root = paths::repo_root();
    let out_dir = paths::resolve("reports/linux_shim_errno_conformance");
    paths::ensure_dir(&out_dir)?;

    let rules: HashMap<&str, Vec<&str>> = HashMap::from([
        ("src/kernel/syscalls/linux_shim/util.rs", vec!["read_user_c_string", "read_user_c_string_allow_empty", "read_user_c_string_array"]),
        ("src/kernel/syscalls/linux_shim/fs/meta.rs", vec!["sys_linux_fstat"]),
        ("src/kernel/syscalls/linux_shim/fs/io/fd_ops.rs", vec!["sys_linux_read", "sys_linux_write"]),
        ("src/kernel/syscalls/linux_shim/fd_process_identity/fd_ops.rs", vec!["sys_linux_pipe", "sys_linux_fcntl"]),
        ("src/kernel/syscalls/linux_shim/fd_process_identity/dir_info.rs", vec!["sys_linux_getdents64", "sys_linux_getcwd", "sys_linux_uname"]),
        ("src/kernel/syscalls/linux_shim/net/epoll.rs", vec!["timeout_ptr_to_retries", "parse_sigmask", "sys_linux_epoll_ctl", "sys_linux_epoll_pwait", "sys_linux_epoll_pwait2"]),
        ("src/kernel/syscalls/linux_shim/net/socket/addr.rs", vec!["read_sockaddr_in", "write_sockaddr_in"]),
        ("src/kernel/syscalls/linux_shim/net/socket/io.rs", vec!["sys_linux_sendto", "sys_linux_recvfrom"]),
        ("src/kernel/syscalls/linux_shim/net/socket/lifecycle.rs", vec!["sys_linux_socketpair"]),
        ("src/kernel/syscalls/linux_shim/net/socket/options.rs", vec!["sys_linux_getsockopt"]),
        ("src/kernel/syscalls/linux_shim/net/msg/compat.rs", vec!["read_linux_msghdr_compat", "read_linux_iovec_compat", "read_sockaddr_in_compat", "write_sockaddr_in_compat", "write_linux_msghdr_namelen_compat", "write_linux_msghdr_flags_compat"]),
        ("src/kernel/syscalls/linux_shim/net/msg/message_ops.rs", vec!["sys_linux_sendmsg", "sys_linux_recvmsg"]),
        ("src/kernel/syscalls/linux_shim/signal.rs", vec!["sys_linux_rt_sigprocmask_shim", "sys_linux_sigaltstack_shim", "sys_linux_rt_sigpending_shim"]),
        ("src/kernel/syscalls/linux_shim/task_time/robust_ops.rs", vec!["sys_linux_get_robust_list"]),
        ("src/kernel/syscalls/linux_shim/task_time/time_ops.rs", vec!["sys_linux_clock_gettime", "sys_linux_clock_nanosleep"]),
        ("src/kernel/syscalls/linux_shim/process/exec.rs", vec!["push_execve_user_word", "prepare_execve_user_stack"]),
    ]);

    let mut results = Vec::new();

    for (rel_path, functions) in &rules {
        let path = root.join(rel_path);
        if !path.exists() {
            for func in functions {
                results.push(ErrnoCheckResult {
                    function: func.to_string(), requirement: "uses EFAULT-only mapping".to_string(),
                    ok: false, file: rel_path.to_string(), detail: "file not found".to_string(),
                });
            }
            continue;
        }

        let text = fs::read_to_string(&path)?;
        let text_norm = normalize_whitespace(&text);

        if text_norm.contains("linux_errno(crate::modules::posix_consts::errno::EACCES)") {
            results.push(ErrnoCheckResult {
                function: "<file-scan>".to_string(), requirement: "no EACCES mapping".to_string(),
                ok: false, file: rel_path.to_string(), detail: "file contains forbidden EACCES token".to_string(),
            });
        }

        for func in functions {
            let body = extract_fn_body(&text, func);
            if body.is_empty() {
                results.push(ErrnoCheckResult {
                    function: func.to_string(), requirement: "uses EFAULT-only mapping".to_string(),
                    ok: false, file: rel_path.to_string(), detail: "function not found".to_string(),
                });
                continue;
            }

            let body_norm = normalize_whitespace(&body);
            let has_efault = body_norm.contains("linux_errno(crate::modules::posix_consts::errno::EFAULT)");
            let has_eacces = body_norm.contains("linux_errno(crate::modules::posix_consts::errno::EACCES)");

            let ok = has_efault && !has_eacces;
            let detail = if !has_efault { "missing EFAULT mapping".to_string() }
                        else if has_eacces { "forbidden EACCES mapping found".to_string() }
                        else { "correctly uses EFAULT mapping".to_string() };

            results.push(ErrnoCheckResult {
                function: func.to_string(), requirement: "uses EFAULT-only mapping".to_string(),
                ok, file: rel_path.to_string(), detail,
            });
        }
    }

    let ok_all = results.iter().all(|r| r.ok);
    let passed = results.iter().filter(|r| r.ok).count();
    let total = results.len();

    let payload = serde_json::json!({
        "summary": { "ok": ok_all, "checks": total, "passed": passed, "failed": total - passed },
        "results": results,
    });

    report::write_json_report(&out_dir.join("summary.json"), &payload)?;
    
    let mut md = format!("# Linux Shim Errno Conformance\n\n- checks: {total}\n- passed: {passed}\n- failed: {}\n\n", total - passed);
    md.push_str("| File | Function | OK | Detail |\n|---|---|---|---|\n");
    for r in &results {
        md.push_str(&format!("| {} | {} | {} | {} |\n", r.file, r.function, if r.ok { "yes" } else { "no" }, r.detail));
    }
    fs::write(out_dir.join("summary.md"), md)?;

    println!("[linux-abi::shim-errno] {}/{} shim checks passed", passed, total);
    Ok(())
}
