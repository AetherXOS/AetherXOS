#!/usr/bin/env python3
"""
P2 gap regression gate.

Fails when actionable marker totals regress beyond configured thresholds.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Dict


def run_cmd(cmd: list[str], cwd: Path) -> int:
    proc = subprocess.run(cmd, cwd=str(cwd), check=False)
    return int(proc.returncode)


def read_json(path: Path) -> Dict[str, Any]:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def load_summary(path: Path) -> Dict[str, Any]:
    payload = read_json(path)
    if not payload:
        return {}
    return payload.get("summary", payload)


def main() -> int:
    parser = argparse.ArgumentParser(description="Run P2 gap regression gate")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--report-json",
        type=Path,
        default=Path("reports/p2_gap/summary.json"),
        help="P2 gap report summary path",
    )
    parser.add_argument(
        "--baseline-json",
        type=Path,
        default=Path("reports/p2_gap/baseline_summary.json"),
        help="Baseline summary path",
    )
    parser.add_argument("--auto-baseline", action="store_true")
    parser.add_argument("--update-baseline-on-success", action="store_true")
    parser.add_argument("--max-actionable-increase", type=int, default=0)
    parser.add_argument("--max-total-increase", type=int, default=2)
    args = parser.parse_args()

    root = args.root.resolve()
    report_json = args.report_json if args.report_json.is_absolute() else root / args.report_json
    baseline_json = (
        args.baseline_json if args.baseline_json.is_absolute() else root / args.baseline_json
    )

    rc = run_cmd(["python", "scripts/p2_gap_report.py"], root)
    if rc != 0:
        print("p2 gap gate: FAIL (report generation failed)")
        return rc

    current = load_summary(report_json)
    baseline = load_summary(baseline_json) if args.auto_baseline or baseline_json.exists() else {}

    failures: list[str] = []
    cur_actionable = int(current.get("actionable_total_markers", 0))
    cur_total = int(current.get("total_markers", 0))

    if baseline:
        base_actionable = int(baseline.get("actionable_total_markers", 0))
        base_total = int(baseline.get("total_markers", 0))
        if cur_actionable > base_actionable + args.max_actionable_increase:
            failures.append(
                f"actionable markers regression: current {cur_actionable} > baseline {base_actionable} + {args.max_actionable_increase}"
            )
        if cur_total > base_total + args.max_total_increase:
            failures.append(
                f"total markers regression: current {cur_total} > baseline {base_total} + {args.max_total_increase}"
            )
    elif args.auto_baseline:
        failures.append(f"missing baseline: {baseline_json}")

    ok = len(failures) == 0
    if ok and args.update_baseline_on_success:
        baseline_json.parent.mkdir(parents=True, exist_ok=True)
        baseline_json.write_text(json.dumps({"summary": current}, indent=2), encoding="utf-8")

    summary = {
        "ok": ok,
        "failures": failures,
        "current": {
            "actionable_total_markers": cur_actionable,
            "total_markers": cur_total,
        },
        "baseline_path": str(baseline_json),
        "baseline_present": bool(baseline),
        "max_actionable_increase": args.max_actionable_increase,
        "max_total_increase": args.max_total_increase,
    }
    out_path = root / "reports" / "p2_gap" / "gate_summary.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps({"summary": summary}, indent=2), encoding="utf-8")
    print(f"p2 gap gate: {'PASS' if ok else 'FAIL'}")
    print(f"summary={out_path}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
