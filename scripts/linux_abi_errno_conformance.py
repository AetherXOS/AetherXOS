#!/usr/bin/env python3
"""
Static errno conformance checks for high-impact linux_compat net ABI handlers.

This script is intentionally conservative: it verifies presence of required
EINVAL-style guards and emits both JSON/Markdown artifacts for CI tracking.
"""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Dict, List


@dataclass
class CheckResult:
    function: str
    requirement: str
    ok: bool
    detail: str


def extract_fn_body(text: str, fn_name: str) -> str:
    needle = f"pub fn {fn_name}("
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


def run_checks(text: str) -> List[CheckResult]:
    reqs: Dict[str, List[tuple[str, List[str]]]] = {
        "sys_linux_epoll_create": [
            ("reject zero size", ["linux_inval()"]),
        ],
        "sys_linux_epoll_pwait": [
            (
                "reject zero/maxevents overflow",
                ["maxevents == 0 || maxevents > MAX_EPOLL_EVENTS"],
            ),
            (
                "reject invalid sigset size",
                ["sigsetsize != linux::SIGSET_SIZE", "parse_optional_sigmask(sigmask, sigsetsize)"],
            ),
        ],
        "sys_linux_ppoll": [
            ("reject poll fd overflow", ["nfds > MAX_POLL_FDS"]),
            (
                "reject invalid timespec",
                [
                    "ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= NANOS_PER_SECOND as i64",
                    "retries_from_timespec(timeout_ptr)",
                ],
            ),
            (
                "reject invalid sigset size",
                ["sigsetsize != linux::SIGSET_SIZE", "parse_optional_sigmask(sigmask, sigsetsize)"],
            ),
        ],
        "sys_linux_pselect6": [
            ("reject fdset overflow", ["nfds > LINUX_FD_SETSIZE"]),
            (
                "reject invalid pselect6 sigset descriptor len",
                ["sig.ss_len != linux::SIGSET_SIZE", "parse_pselect6_sigmask(sigmask_arg)"],
            ),
            (
                "reject invalid timespec",
                [
                    "ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= NANOS_PER_SECOND as i64",
                    "retries_from_timespec(timeout)",
                ],
            ),
        ],
        "sys_linux_epoll_pwait2": [
            (
                "reject invalid timespec",
                ["ts.tv_sec < 0 || ts.tv_nsec < 0 || ts.tv_nsec >= 1_000_000_000"],
            ),
        ],
    }

    results: List[CheckResult] = []
    for fn_name, checks in reqs.items():
        body = extract_fn_body(text, fn_name)
        if not body:
            for req_name, _ in checks:
                results.append(
                    CheckResult(
                        function=fn_name,
                        requirement=req_name,
                        ok=False,
                        detail="function not found",
                    )
                )
            continue

        body_norm = " ".join(body.split())
        for req_name, tokens in checks:
            ok = any(token in body_norm for token in tokens)
            results.append(
                CheckResult(
                    function=fn_name,
                    requirement=req_name,
                    ok=ok,
                    detail=("matched" if ok else f"missing tokens: {', '.join(tokens)}"),
                )
            )

    return results


def to_md(results: List[CheckResult]) -> str:
    ok_count = sum(1 for r in results if r.ok)
    total = len(results)
    lines = [
        "# Linux ABI Errno Conformance (Static)",
        "",
        f"- checks: {total}",
        f"- passed: {ok_count}",
        f"- failed: {total - ok_count}",
        "",
        "| Function | Requirement | OK | Detail |",
        "|---|---|---|---|",
    ]
    for r in results:
        lines.append(
            f"| {r.function} | {r.requirement} | {'yes' if r.ok else 'no'} | {r.detail} |"
        )
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Linux ABI errno conformance static checks")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--out-dir", type=Path, default=Path("reports/errno_conformance"))
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    target = root / "src/modules/linux_compat/net/poll.rs"
    text = target.read_text(encoding="utf-8")

    results = run_checks(text)
    ok = all(r.ok for r in results)

    summary = {
        "ok": ok,
        "checks": len(results),
        "passed": sum(1 for r in results if r.ok),
        "failed": sum(1 for r in results if not r.ok),
        "target": str(target).replace("\\", "/"),
    }

    payload = {
        "summary": summary,
        "results": [asdict(r) for r in results],
    }

    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    (out_dir / "summary.md").write_text(to_md(results), encoding="utf-8")

    print(f"linux abi errno conformance: {'PASS' if ok else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
