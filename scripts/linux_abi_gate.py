#!/usr/bin/env python3
"""
Linux ABI quality gate.

Runs full + active ABI inventories/readiness checks and fails on regressions
based on configurable thresholds.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Dict, List, Tuple


CRITICAL_SYSCALLS = {
    "read",
    "write",
    "open",
    "openat",
    "close",
    "lseek",
    "getdents64",
    "readdir",
    "readv",
    "writev",
    "preadv",
    "pwritev",
    "fork",
    "clone",
    "clone3",
    "execve",
    "execveat",
    "wait4",
    "waitpid",
    "exit",
    "exit_group",
    "mmap",
    "mmap2",
    "munmap",
    "brk",
    "mprotect",
    "mremap",
    "rt_sigaction",
    "rt_sigprocmask",
    "rt_sigpending",
    "rt_sigtimedwait",
    "sigaltstack",
}


def run_cmd(cmd: List[str], cwd: Path) -> Tuple[int, str, str]:
    proc = subprocess.run(
        cmd,
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )
    return int(proc.returncode), proc.stdout, proc.stderr


def load_json(path: Path) -> Dict[str, Any]:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def load_summary(path: Path) -> Dict[str, Any]:
    payload = load_json(path)
    if not payload:
        return {}
    return payload.get("summary", payload)


def normalize_syscall_name(function_name: str) -> str:
    if function_name.startswith("sys_linux_"):
        return function_name[len("sys_linux_") :]
    return function_name


def compute_critical_blockers(gap_summary_path: Path) -> int:
    payload = load_json(gap_summary_path)
    entries = payload.get("entries", [])
    count = 0
    for entry in entries:
        if entry.get("category") != "stub":
            continue
        fn = str(entry.get("function", ""))
        if normalize_syscall_name(fn) in CRITICAL_SYSCALLS:
            count += 1
    return count


def main() -> int:
    parser = argparse.ArgumentParser(description="Linux ABI quality gate")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("reports/linux_abi_gate"),
        help="Gate artifact output directory",
    )
    parser.add_argument("--min-full-readiness", type=float, default=100.0)
    parser.add_argument("--min-active-readiness", type=float, default=100.0)
    parser.add_argument("--max-full-gaps", type=int, default=0)
    parser.add_argument("--max-active-gaps", type=int, default=0)
    parser.add_argument("--max-critical-blockers", type=int, default=0)
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    commands = [
        ["python", "scripts/linux_abi_gap_inventory.py", "--out-dir", "reports/abi_gap_inventory_full"],
        [
            "python",
            "scripts/linux_abi_readiness_score.py",
            "--out-dir",
            "reports/linux_abi_readiness_full",
            "--gap-summary",
            "reports/abi_gap_inventory_full/summary.json",
        ],
        ["python", "scripts/linux_abi_gap_inventory.py", "--out-dir", "reports/abi_gap_inventory", "--linux-compat-only"],
        [
            "python",
            "scripts/linux_abi_readiness_score.py",
            "--out-dir",
            "reports/linux_abi_readiness",
            "--gap-summary",
            "reports/abi_gap_inventory/summary.json",
        ],
    ]

    steps: List[Dict[str, Any]] = []
    for cmd in commands:
        rc, stdout, stderr = run_cmd(cmd, root)
        steps.append(
            {
                "cmd": cmd,
                "return_code": rc,
                "ok": rc == 0,
                "stdout_tail": "\n".join(stdout.splitlines()[-40:]),
                "stderr_tail": "\n".join(stderr.splitlines()[-40:]),
            }
        )
        if rc != 0:
            summary = {
                "ok": False,
                "reason": "subcommand failed",
                "steps": steps,
            }
            (out_dir / "summary.json").write_text(
                json.dumps({"summary": summary}, indent=2),
                encoding="utf-8",
            )
            print("linux abi gate: FAIL")
            print(f"summary={out_dir / 'summary.json'}")
            return 1

    full_gap_summary_path = root / "reports/abi_gap_inventory_full/summary.json"
    active_gap_summary_path = root / "reports/abi_gap_inventory/summary.json"
    full_readiness_summary_path = root / "reports/linux_abi_readiness_full/summary.json"
    active_readiness_summary_path = root / "reports/linux_abi_readiness/summary.json"

    full_gap = load_summary(full_gap_summary_path)
    active_gap = load_summary(active_gap_summary_path)
    full_ready = load_summary(full_readiness_summary_path)
    active_ready = load_summary(active_readiness_summary_path)

    full_gaps = int(full_gap.get("total_gaps", 0))
    active_gaps = int(active_gap.get("total_gaps", 0))
    full_readiness = float(full_ready.get("score", 0.0))
    active_readiness = float(active_ready.get("score", 0.0))
    critical_blockers = compute_critical_blockers(full_gap_summary_path)

    failures: List[str] = []
    if full_gaps > args.max_full_gaps:
        failures.append(f"full gaps {full_gaps} > max {args.max_full_gaps}")
    if active_gaps > args.max_active_gaps:
        failures.append(f"active gaps {active_gaps} > max {args.max_active_gaps}")
    if full_readiness < args.min_full_readiness:
        failures.append(
            f"full readiness {full_readiness:.1f} < min {args.min_full_readiness:.1f}"
        )
    if active_readiness < args.min_active_readiness:
        failures.append(
            f"active readiness {active_readiness:.1f} < min {args.min_active_readiness:.1f}"
        )
    if critical_blockers > args.max_critical_blockers:
        failures.append(
            f"critical blockers {critical_blockers} > max {args.max_critical_blockers}"
        )

    ok = len(failures) == 0
    summary = {
        "ok": ok,
        "thresholds": {
            "min_full_readiness": args.min_full_readiness,
            "min_active_readiness": args.min_active_readiness,
            "max_full_gaps": args.max_full_gaps,
            "max_active_gaps": args.max_active_gaps,
            "max_critical_blockers": args.max_critical_blockers,
        },
        "metrics": {
            "full_gaps": full_gaps,
            "active_gaps": active_gaps,
            "full_readiness": full_readiness,
            "active_readiness": active_readiness,
            "critical_blockers": critical_blockers,
        },
        "failures": failures,
        "steps": steps,
    }

    (out_dir / "summary.json").write_text(
        json.dumps({"summary": summary}, indent=2),
        encoding="utf-8",
    )

    md_lines = [
        "# Linux ABI Gate",
        "",
        f"- ok: `{ok}`",
        f"- full_gaps: `{full_gaps}`",
        f"- active_gaps: `{active_gaps}`",
        f"- full_readiness: `{full_readiness:.1f}`",
        f"- active_readiness: `{active_readiness:.1f}`",
        f"- critical_blockers: `{critical_blockers}`",
    ]
    if failures:
        md_lines += ["", "## Failures", ""]
        md_lines += [f"- {msg}" for msg in failures]

    (out_dir / "summary.md").write_text("\n".join(md_lines) + "\n", encoding="utf-8")

    print(f"linux abi gate: {'PASS' if ok else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
