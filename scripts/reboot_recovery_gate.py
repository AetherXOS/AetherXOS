#!/usr/bin/env python3
"""
Reboot recovery gate over QEMU soak outputs.

Checks:
1) At least N successful non-chaos boots.
2) For each chaos round, at least one subsequent successful non-chaos boot
   exists within a recovery window.
3) Crash pipeline over captured logs is monotonic.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Dict, List


def load_json(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Gate reboot recovery from soak matrix artifacts")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--soak-summary",
        type=Path,
        default=Path("artifacts/qemu_soak/summary.json"),
        help="Path to qemu_soak_matrix summary.json",
    )
    parser.add_argument(
        "--min-successful-boots",
        type=int,
        default=3,
        help="Minimum successful non-chaos rounds required",
    )
    parser.add_argument(
        "--recovery-window",
        type=int,
        default=3,
        help="Max rounds after chaos round to observe a successful recovery boot",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("reports/reboot_recovery_gate"),
        help="Output directory for gate reports",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    soak_summary_path = args.soak_summary if args.soak_summary.is_absolute() else root / args.soak_summary
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    if not soak_summary_path.exists():
        print(f"reboot recovery gate: FAIL (missing {soak_summary_path})")
        return 1

    soak = load_json(soak_summary_path)
    raw_rounds = soak.get("rounds", [])
    if not isinstance(raw_rounds, list):
        print("reboot recovery gate: FAIL (invalid soak summary format: rounds must be list)")
        return 1
    rounds: List[Dict[str, Any]] = raw_rounds
    if not rounds:
        print("reboot recovery gate: FAIL (no rounds in soak summary)")
        return 1

    successful_rounds = [
        r for r in rounds if r.get("expected_success", False) and r.get("ok", False)
    ]
    chaos_rounds = [r for r in rounds if not r.get("expected_success", False)]

    failures: List[str] = []
    if len(successful_rounds) < args.min_successful_boots:
        failures.append(
            f"successful non-chaos rounds {len(successful_rounds)} < {args.min_successful_boots}"
        )

    for chaos in chaos_rounds:
        idx = int(chaos["round_index"])
        recovered = False
        for cand in rounds:
            cidx = int(cand["round_index"])
            if cidx <= idx:
                continue
            if cidx > idx + args.recovery_window:
                break
            if cand.get("expected_success", False) and cand.get("ok", False):
                recovered = True
                break
        if not recovered:
            failures.append(
                f"no successful recovery boot within {args.recovery_window} rounds after chaos round {idx}"
            )

    # Run crash recovery pipeline over the same log directory.
    logs_dir = soak_summary_path.parent
    crash_out_dir = out_dir / "crash_pipeline"
    cmd = [
        "python",
        "scripts/crash_recovery_pipeline.py",
        "--logs-dir",
        str(logs_dir),
        "--out-dir",
        str(crash_out_dir),
    ]
    proc = subprocess.run(cmd, cwd=str(root), capture_output=True, text=True, check=False)
    crash_ok = proc.returncode == 0
    if not crash_ok:
        failures.append("crash_recovery_pipeline failed")

    summary = {
        "ok": len(failures) == 0,
        "min_successful_boots": args.min_successful_boots,
        "recovery_window": args.recovery_window,
        "successful_rounds": len(successful_rounds),
        "chaos_rounds": len(chaos_rounds),
        "failures": failures,
        "crash_pipeline_ok": crash_ok,
    }
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")

    md = [
        "# Reboot Recovery Gate",
        "",
        f"- ok: `{summary['ok']}`",
        f"- successful_rounds: `{summary['successful_rounds']}`",
        f"- chaos_rounds: `{summary['chaos_rounds']}`",
        f"- crash_pipeline_ok: `{summary['crash_pipeline_ok']}`",
        "",
    ]
    if failures:
        md.append("## Failures")
        md.append("")
        for fail in failures:
            md.append(f"- {fail}")
    else:
        md.append("All reboot-recovery gate checks passed.")
    (out_dir / "summary.md").write_text("\n".join(md) + "\n", encoding="utf-8")

    print(f"reboot recovery gate: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
