#!/usr/bin/env python3
"""
Crash-dump and recovery pipeline helper.

Pipeline:
1) Parse each crash/serial log with crash_artifacts_report.py
2) Aggregate panic/event metrics
3) Emit pipeline summary for CI/manual review
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any, Dict, List


def run_crash_report(root: Path, log_file: Path, out_json: Path) -> None:
    cmd = [
        "python",
        "scripts/crash_artifacts_report.py",
        "--log",
        str(log_file),
        "--format",
        "json",
        "--out",
        str(out_json),
    ]
    proc = subprocess.run(
        cmd,
        cwd=str(root),
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"crash_artifacts_report failed for {log_file}: rc={proc.returncode}\n{proc.stderr}"
        )


def load_json(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Run crash/recovery artifact pipeline")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--logs-dir",
        type=Path,
        default=Path("artifacts/crash"),
        help="Directory containing captured kernel logs (*.log)",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("reports/crash_pipeline"),
        help="Output directory for per-log and summary reports",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    logs_dir = args.logs_dir if args.logs_dir.is_absolute() else root / args.logs_dir
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    logs = sorted(logs_dir.glob("*.log"))
    if not logs:
        summary = {
            "ok": False,
            "reason": f"no .log files found in {logs_dir}",
            "logs_processed": 0,
        }
        (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
        (out_dir / "summary.md").write_text(
            "# Crash Recovery Pipeline\n\n- ok: `False`\n- reason: no crash logs found\n",
            encoding="utf-8",
        )
        print("crash pipeline: FAIL (no logs)")
        return 1

    parsed_reports: List[Dict[str, Any]] = []
    for log_file in logs:
        out_json = out_dir / f"{log_file.stem}.json"
        run_crash_report(root, log_file, out_json)
        parsed_reports.append(load_json(out_json))

    panic_counts = [int(r.get("panic", {}).get("panic_count", 0)) for r in parsed_reports]
    latest_seqs = [int(r.get("latest_seq", 0)) for r in parsed_reports]
    total_events = [int(r.get("event_count", 0)) for r in parsed_reports]

    monotonic_panic = all(
        panic_counts[i] <= panic_counts[i + 1] for i in range(len(panic_counts) - 1)
    )
    monotonic_seq = all(
        latest_seqs[i] <= latest_seqs[i + 1] for i in range(len(latest_seqs) - 1)
    )

    summary = {
        "ok": monotonic_panic and monotonic_seq,
        "logs_processed": len(logs),
        "panic_counts": panic_counts,
        "latest_seqs": latest_seqs,
        "total_events": total_events,
        "checks": {
            "panic_count_monotonic": monotonic_panic,
            "latest_seq_monotonic": monotonic_seq,
        },
        "logs": [str(p) for p in logs],
    }

    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    md = [
        "# Crash Recovery Pipeline",
        "",
        f"- ok: `{summary['ok']}`",
        f"- logs_processed: `{summary['logs_processed']}`",
        f"- panic_count_monotonic: `{summary['checks']['panic_count_monotonic']}`",
        f"- latest_seq_monotonic: `{summary['checks']['latest_seq_monotonic']}`",
        "",
        "## Logs",
        "",
    ]
    for p in logs:
        md.append(f"- `{p}`")
    (out_dir / "summary.md").write_text("\n".join(md) + "\n", encoding="utf-8")

    print(f"crash pipeline: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
