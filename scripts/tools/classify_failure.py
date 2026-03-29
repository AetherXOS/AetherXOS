#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path


RULES = [
    ("qemu-system-x86_64 not found", "missing_qemu", "high", "Run scripts/hypercore.ps1 -Command install"),
    ("xorriso not found", "missing_xorriso", "high", "Run scripts/hypercore.ps1 -Command install"),
    ("limine binaries", "missing_limine", "high", "Run scripts/setup/setup_limine.ps1"),
    ("cargo not found", "missing_rust", "high", "Run scripts/setup/setup_rust.ps1"),
    ("python not found", "missing_python", "high", "Install Python and retry"),
    ("timeout", "runtime_timeout", "medium", "Increase timeout or reduce rounds"),
    ("panic", "kernel_panic", "high", "Inspect round log and panic report"),
    ("baseline", "baseline_regression", "medium", "Update baseline only after root-cause analysis"),
    ("permission", "permission_issue", "medium", "Run elevated shell or fix file permissions"),
]


def classify(message: str) -> dict:
    lower = message.lower()
    for needle, code, severity, hint in RULES:
        if needle in lower:
            return {
                "category": code,
                "severity": severity,
                "hint": hint,
            }
    return {
        "category": "unknown",
        "severity": "medium",
        "hint": "Inspect command output and logs",
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="Classify HyperCore tooling failures")
    ap.add_argument("--message", default="")
    ap.add_argument("--file", type=Path, default=None)
    args = ap.parse_args()

    msg = args.message
    if args.file is not None and args.file.exists():
        msg = args.file.read_text(encoding="utf-8", errors="replace")

    result = classify(msg or "")
    result["message_excerpt"] = (msg or "")[:300]
    print(json.dumps(result, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
