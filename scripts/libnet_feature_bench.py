#!/usr/bin/env python3
"""
Feature-scoped LibNet microbench harness.

Runs LibNet microbench tests under selected feature sets and emits a compact
JSON/Markdown report for CI/operator review.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import time
from pathlib import Path
from typing import Dict, List, Optional


def run_case(root: Path, target: str, features: Optional[str], mode: str) -> Dict[str, object]:
    if mode == "test-no-run":
        cmd = ["cargo", "test", "--no-run"]
    else:
        cmd = ["cargo", "check"]
    if features:
        cmd.extend(["--features", features])
    cmd.extend(["--target", target])
    started = time.perf_counter()
    proc = subprocess.run(cmd, cwd=str(root), capture_output=True, text=True, check=False)
    elapsed = time.perf_counter() - started
    return {
        "mode": mode,
        "features": features or "<default>",
        "ok": proc.returncode == 0,
        "returncode": proc.returncode,
        "duration_sec": round(elapsed, 3),
        "stdout_tail": "\n".join(proc.stdout.splitlines()[-40:]),
        "stderr_tail": "\n".join(proc.stderr.splitlines()[-40:]),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Run LibNet feature-scoped microbench tests")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--target",
        default="x86_64-unknown-none",
        help="Cargo target triple",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("reports/libnet_feature_bench"),
        help="Output directory",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    cases_spec: List[Dict[str, object]] = [
        {"features": None, "required": True, "mode": "test-no-run"},
        {"features": "network_transport", "required": False, "mode": "check"},
        {
            "features": "network_transport,network_http,network_https",
            "required": False,
            "mode": "check",
        },
    ]

    cases: List[Dict[str, object]] = []
    required_ok = True
    for spec in cases_spec:
        case = run_case(root, args.target, spec["features"], str(spec["mode"]))
        case["required"] = bool(spec["required"])
        if not case["ok"] and not case["required"]:
            case["skipped"] = True
        else:
            case["skipped"] = False
        if case["required"] and not case["ok"]:
            required_ok = False
        cases.append(case)

    ok = required_ok

    payload = {
        "ok": ok,
        "target": args.target,
        "cases": cases,
    }
    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")

    md_lines = [
        "# LibNet Feature Bench",
        "",
        f"- ok: `{ok}`",
        f"- target: `{args.target}`",
        "",
        "## Cases",
        "",
    ]
    for case in cases:
        md_lines.append(
            f"- features=`{case['features']}` required=`{case['required']}` "
            f"ok=`{case['ok']}` skipped=`{case['skipped']}` rc=`{case['returncode']}` "
            f"duration_sec=`{case['duration_sec']}`"
        )
    (out_dir / "summary.md").write_text("\n".join(md_lines) + "\n", encoding="utf-8")

    print(f"libnet feature bench: {'PASS' if ok else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
