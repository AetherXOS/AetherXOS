#!/usr/bin/env python3
"""
Compute a unified Linux ABI readiness score from existing report artifacts.

Inputs (default locations under repo root):
- reports/errno_conformance/summary.json
- reports/linux_shim_errno_conformance/summary.json
- reports/abi_gap_inventory/summary.json
- reports/syscall_coverage/summary.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict


def load_json(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def safe_div(num: float, den: float) -> float:
    if den <= 0:
        return 0.0
    return num / den


def clamp01(value: float) -> float:
    if value < 0.0:
        return 0.0
    if value > 1.0:
        return 1.0
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description="Compute unified Linux ABI readiness score")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--out-dir", type=Path, default=Path("reports/abi_readiness"))
    parser.add_argument(
        "--gap-summary",
        type=Path,
        default=Path("reports/abi_gap_inventory/summary.json"),
        help="Path to ABI gap inventory summary.json",
    )
    parser.add_argument(
        "--coverage-summary",
        type=Path,
        default=Path("reports/syscall_coverage/summary.json"),
        help="Path to syscall coverage summary.json",
    )
    parser.add_argument(
        "--min-score",
        type=float,
        default=None,
        help="Fail if computed score is below this threshold (0..100)",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    errno_summary = load_json(root / "reports/errno_conformance/summary.json")["summary"]
    shim_summary = load_json(root / "reports/linux_shim_errno_conformance/summary.json")["summary"]
    gap_path = args.gap_summary if args.gap_summary.is_absolute() else root / args.gap_summary
    cov_path = (
        args.coverage_summary
        if args.coverage_summary.is_absolute()
        else root / args.coverage_summary
    )
    gap_summary = load_json(gap_path)["summary"]
    cov_summary = load_json(cov_path)

    errno_pass = clamp01(
        safe_div(float(errno_summary.get("passed", 0)), float(errno_summary.get("checks", 0)))
    )
    shim_pass = clamp01(
        safe_div(float(shim_summary.get("passed", 0)), float(shim_summary.get("checks", 0)))
    )

    total_gaps = float(gap_summary.get("total_gaps", 0))
    stub_count = float(gap_summary.get("stub_count", 0))
    partial_count = float(gap_summary.get("partial_or_feature_gated_count", 0))
    gap_penalty = clamp01(safe_div(stub_count * 1.0 + partial_count * 0.5, total_gaps if total_gaps > 0 else 1.0))
    gap_score = 1.0 - gap_penalty

    implemented_pct = float(cov_summary.get("implemented_pct", 0.0))
    coverage_score = clamp01(implemented_pct / 100.0)

    # Weighted score prioritizing semantic safety + coverage.
    score_0_1 = (
        0.35 * errno_pass
        + 0.25 * shim_pass
        + 0.20 * gap_score
        + 0.20 * coverage_score
    )
    score = round(score_0_1 * 100.0, 2)

    payload = {
        "summary": {
            "score": score,
            "weights": {
                "errno_conformance": 0.35,
                "linux_shim_errno_conformance": 0.25,
                "abi_gap_inventory": 0.20,
                "syscall_coverage": 0.20,
            },
            "components": {
                "errno_pass_ratio": round(errno_pass, 5),
                "linux_shim_pass_ratio": round(shim_pass, 5),
                "abi_gap_score": round(gap_score, 5),
                "syscall_coverage_score": round(coverage_score, 5),
            },
        }
    }

    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    (out_dir / "summary.md").write_text(
        "\n".join(
            [
                "# Linux ABI Readiness Score",
                "",
                f"- score: {score}",
                f"- errno_pass_ratio: {payload['summary']['components']['errno_pass_ratio']}",
                f"- linux_shim_pass_ratio: {payload['summary']['components']['linux_shim_pass_ratio']}",
                f"- abi_gap_score: {payload['summary']['components']['abi_gap_score']}",
                f"- syscall_coverage_score: {payload['summary']['components']['syscall_coverage_score']}",
                "",
            ]
        ),
        encoding="utf-8",
    )

    print(f"linux abi readiness score: {score}")
    print(f"summary={out_dir / 'summary.json'}")

    if args.min_score is not None and score < args.min_score:
        print(f"linux abi readiness gate: FAIL (score={score} < min_score={args.min_score})")
        return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
