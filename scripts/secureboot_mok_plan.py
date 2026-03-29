#!/usr/bin/env python3
"""
Generate MOK enrollment runbook and command plan.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate MOK enrollment plan")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--cert", type=Path, default=Path("keys/MOK.cer"))
    parser.add_argument("--out-dir", type=Path, default=Path("reports/secureboot"))
    args = parser.parse_args()

    root = args.root.resolve()
    cert = args.cert if args.cert.is_absolute() else root / args.cert
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    summary = {
        "generated_utc": utc_now(),
        "cert_path": str(cert),
        "steps": [
            "1) Copy certificate to target machine.",
            "2) Run: mokutil --import <cert>",
            "3) Reboot and enroll key in MOK Manager UI.",
            "4) Verify: mokutil --list-enrolled",
            "5) Reboot and validate shim/grub/loader path.",
        ],
        "commands": [
            f"mokutil --import {cert}",
            "mokutil --list-enrolled",
            "mokutil --test-key <cert>",
        ],
        "note": "This plan assumes shim/MOK flow; actual command availability depends on distro/firmware.",
    }

    (out_dir / "mok_plan.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    md = [
        "# Secure Boot MOK Enrollment Plan",
        "",
        f"- generated_utc: `{summary['generated_utc']}`",
        f"- cert_path: `{summary['cert_path']}`",
        "",
        "## Steps",
        "",
    ]
    md.extend([f"- {s}" for s in summary["steps"]])
    md.extend(["", "## Commands", ""])
    md.extend([f"- `{c}`" for c in summary["commands"]])
    (out_dir / "mok_plan.md").write_text("\n".join(md) + "\n", encoding="utf-8")
    print("secureboot-mok-plan: PASS")
    print(f"plan={out_dir / 'mok_plan.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

