#!/usr/bin/env python3
"""
Static errno conformance checks for linux_shim user-pointer fault mapping.

Goal: ensure user memory access failures in selected linux_shim paths map to EFAULT
instead of EACCES for Linux ABI compatibility.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path


@dataclass
class CheckResult:
    file: str
    function: str
    ok: bool
    detail: str


TARGET_FILES = [
    # kept for high-level scan in main via FN_RULES
]

FN_RULES = {
    "src/kernel/syscalls/linux_shim/util.rs": [
        "read_user_c_string",
        "read_user_c_string_allow_empty",
        "read_user_c_string_array",
    ],
    "src/kernel/syscalls/linux_shim/fs/meta.rs": ["sys_linux_fstat"],
    "src/kernel/syscalls/linux_shim/fs/io/fd_ops.rs": ["sys_linux_read", "sys_linux_write"],
    "src/kernel/syscalls/linux_shim/fd_process_identity/fd_ops.rs": ["sys_linux_pipe", "sys_linux_fcntl"],
    "src/kernel/syscalls/linux_shim/fd_process_identity/dir_info.rs": [
        "sys_linux_getdents64",
        "sys_linux_getcwd",
        "sys_linux_uname",
    ],
    "src/kernel/syscalls/linux_shim/net/epoll.rs": [
        "timeout_ptr_to_retries",
        "parse_sigmask",
        "sys_linux_epoll_ctl",
        "sys_linux_epoll_pwait",
        "sys_linux_epoll_pwait2",
    ],
    "src/kernel/syscalls/linux_shim/net/socket/addr.rs": ["read_sockaddr_in", "write_sockaddr_in"],
    "src/kernel/syscalls/linux_shim/net/socket/io.rs": ["sys_linux_sendto", "sys_linux_recvfrom"],
    "src/kernel/syscalls/linux_shim/net/socket/lifecycle.rs": ["sys_linux_socketpair"],
    "src/kernel/syscalls/linux_shim/net/socket/options.rs": ["sys_linux_getsockopt"],
    "src/kernel/syscalls/linux_shim/net/msg/compat.rs": [
        "read_linux_msghdr_compat",
        "read_linux_iovec_compat",
        "read_sockaddr_in_compat",
        "write_sockaddr_in_compat",
        "write_linux_msghdr_namelen_compat",
        "write_linux_msghdr_flags_compat",
    ],
    "src/kernel/syscalls/linux_shim/net/msg/message_ops.rs": ["sys_linux_sendmsg", "sys_linux_recvmsg"],
    "src/kernel/syscalls/linux_shim/signal.rs": [
        "sys_linux_rt_sigprocmask_shim",
        "sys_linux_sigaltstack_shim",
        "sys_linux_rt_sigpending_shim",
    ],
    "src/kernel/syscalls/linux_shim/task_time/robust_ops.rs": ["sys_linux_get_robust_list"],
    "src/kernel/syscalls/linux_shim/task_time/time_ops.rs": [
        "sys_linux_clock_gettime",
        "sys_linux_clock_nanosleep",
    ],
    "src/kernel/syscalls/linux_shim/process/exec.rs": [
        "push_execve_user_word",
        "prepare_execve_user_stack",
    ],
}


def extract_fn_body(text: str, fn_name: str) -> str:
    needle = f"fn {fn_name}("
    start = text.find(needle)
    if start < 0:
        return ""
    brace = text.find("{", start)
    if brace < 0:
        return ""
    depth = 0
    i = brace
    while i < len(text):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[brace : i + 1]
        i += 1
    return ""


def check_function(file_rel: str, text: str, fn_name: str) -> CheckResult:
    body = extract_fn_body(text, fn_name)
    if not body:
        return CheckResult(file=file_rel, function=fn_name, ok=False, detail="function not found")

    body_norm = " ".join(body.split())
    has_efault = "linux_errno(crate::modules::posix_consts::errno::EFAULT)" in body_norm
    has_eacces = "linux_errno(crate::modules::posix_consts::errno::EACCES)" in body_norm
    if not has_efault:
        return CheckResult(file=file_rel, function=fn_name, ok=False, detail="missing EFAULT mapping token in function body")
    if has_eacces:
        return CheckResult(file=file_rel, function=fn_name, ok=False, detail="found forbidden EACCES mapping token in function body")
    return CheckResult(file=file_rel, function=fn_name, ok=True, detail="function body uses EFAULT-only mapping")


def to_md(results: list[CheckResult]) -> str:
    passed = sum(1 for r in results if r.ok)
    total = len(results)
    lines = [
        "# Linux Shim Errno Conformance (Static)",
        "",
        f"- checks: {total}",
        f"- passed: {passed}",
        f"- failed: {total - passed}",
        "",
        "| File | Function | OK | Detail |",
        "|---|---|---|---|",
    ]
    for r in results:
        lines.append(f"| {r.file} | {r.function} | {'yes' if r.ok else 'no'} | {r.detail} |")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="linux_shim errno conformance static checks")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--out-dir", type=Path, default=Path("reports/linux_shim_errno_conformance"))
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    results: list[CheckResult] = []
    for rel, functions in FN_RULES.items():
        path = root / rel
        if not path.exists():
            for fn_name in functions:
                results.append(CheckResult(file=rel, function=fn_name, ok=False, detail="file not found"))
            continue

        text = path.read_text(encoding="utf-8")
        text_norm = " ".join(text.split())
        if "linux_errno(crate::modules::posix_consts::errno::EACCES)" in text_norm:
            results.append(
                CheckResult(
                    file=rel,
                    function="<file-scan>",
                    ok=False,
                    detail="file still contains forbidden EACCES mapping token",
                )
            )
        for fn_name in functions:
            results.append(check_function(rel, text, fn_name))
    ok = all(r.ok for r in results)

    payload = {
        "summary": {
            "ok": ok,
            "checks": len(results),
            "passed": sum(1 for r in results if r.ok),
            "failed": sum(1 for r in results if not r.ok),
        },
        "results": [asdict(r) for r in results],
    }

    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    (out_dir / "summary.md").write_text(to_md(results), encoding="utf-8")

    print(f"linux shim errno conformance: {'PASS' if ok else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
