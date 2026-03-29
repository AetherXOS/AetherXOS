#!/usr/bin/env python3
"""
Soak/stress/chaos orchestration for HyperCore host-side validation.

This runner executes repeatable command rounds and injects lightweight chaos
variables (incremental on/off, test thread count) to expose flaky behavior.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import subprocess
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Dict, List

SYSCALL_GATE_MIN_IMPLEMENTED_PCT = "100"
SYSCALL_GATE_MAX_NO_DEFAULT = "0"
SYSCALL_GATE_MAX_NO_LINUX_COMPAT = "0"
SYSCALL_GATE_MAX_PARTIAL_DEFAULT = "0"
SYSCALL_GATE_MAX_PARTIAL_LINUX_COMPAT = "0"
SYSCALL_GATE_MAX_EXTERNAL = "0"


@dataclass
class RoundResult:
    round_index: int
    scenario: str
    command: List[str]
    duration_sec: float
    ok: bool
    return_code: int
    chaos: Dict[str, str]
    stdout_tail: str
    stderr_tail: str


def syscall_coverage_cmd(
    *,
    report_md: str,
    report_json: str,
    max_no: str,
    max_partial: str,
    linux_compat_enabled: bool,
) -> list[str]:
    cmd = [
        "python",
        "scripts/syscall_coverage_report.py",
    ]
    if linux_compat_enabled:
        cmd.append("--linux-compat-enabled")
    cmd.extend(
        [
            "--format",
            "md",
            "--out",
            report_md,
            "--summary-out",
            report_json,
            "--min-implemented-pct",
            SYSCALL_GATE_MIN_IMPLEMENTED_PCT,
            "--max-no",
            max_no,
            "--max-partial",
            max_partial,
            "--max-external",
            SYSCALL_GATE_MAX_EXTERNAL,
        ]
    )
    return cmd


def run_cmd(
    cmd: List[str],
    cwd: Path,
    env: Dict[str, str],
    timeout_sec: int,
) -> tuple[bool, int, float, str, str]:
    start = time.perf_counter()
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(cwd),
            env=env,
            capture_output=True,
            text=True,
            timeout=timeout_sec,
            check=False,
        )
        dur = time.perf_counter() - start
        return proc.returncode == 0, proc.returncode, dur, proc.stdout, proc.stderr
    except subprocess.TimeoutExpired as exc:
        dur = time.perf_counter() - start
        out = exc.stdout or ""
        err = exc.stderr or ""
        return False, 124, dur, out, err


def main() -> int:
    parser = argparse.ArgumentParser(description="Run soak/stress/chaos validation rounds")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument("--rounds", type=int, default=50, help="Total validation rounds")
    parser.add_argument(
        "--timeout-sec",
        type=int,
        default=240,
        help="Per-round command timeout seconds",
    )
    parser.add_argument(
        "--seed",
        type=int,
        default=20260305,
        help="Deterministic RNG seed for chaos choices",
    )
    parser.add_argument(
        "--report-json",
        type=Path,
        default=Path("reports/soak_stress_chaos.json"),
        help="Output JSON report path",
    )
    parser.add_argument(
        "--report-md",
        type=Path,
        default=Path("reports/soak_stress_chaos.md"),
        help="Output Markdown report path",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    random.seed(args.seed)

    scenarios: list[tuple[str, list[str]]] = [
        ("cargo-check", ["cargo", "check"]),
        ("cargo-test-no-run", ["cargo", "test", "--no-run"]),
        ("driver-config-smoke", ["python", "scripts/driver_config_smoke.py"]),
        (
            "syscall-coverage",
            syscall_coverage_cmd(
                report_md="reports/syscall_coverage.md",
                report_json="reports/syscall_coverage_summary.json",
                max_no=SYSCALL_GATE_MAX_NO_DEFAULT,
                max_partial=SYSCALL_GATE_MAX_PARTIAL_DEFAULT,
                linux_compat_enabled=False,
            ),
        ),
        (
            "syscall-coverage-linux-compat",
            [
                "cargo",
                "check",
                "--features",
                "linux_compat,posix_deep_tests",
            ],
        ),
        (
            "syscall-coverage-linux-compat-report",
            syscall_coverage_cmd(
                report_md="reports/syscall_coverage_linux_compat.md",
                report_json="reports/syscall_coverage_linux_compat_summary.json",
                max_no=SYSCALL_GATE_MAX_NO_LINUX_COMPAT,
                max_partial=SYSCALL_GATE_MAX_PARTIAL_LINUX_COMPAT,
                linux_compat_enabled=True,
            ),
        ),
    ]

    results: list[RoundResult] = []
    for idx in range(1, args.rounds + 1):
        scenario_name, cmd = random.choice(scenarios)
        chaos = {
            "CARGO_INCREMENTAL": random.choice(["0", "1"]),
            "RUST_TEST_THREADS": random.choice(["1", "2", "4", "8"]),
        }
        env = dict(os.environ)
        env.update(chaos)

        ok, rc, dur, out, err = run_cmd(cmd, root, env, args.timeout_sec)
        results.append(
            RoundResult(
                round_index=idx,
                scenario=scenario_name,
                command=cmd,
                duration_sec=dur,
                ok=ok,
                return_code=rc,
                chaos=chaos,
                stdout_tail="\n".join(out.splitlines()[-20:]),
                stderr_tail="\n".join(err.splitlines()[-20:]),
            )
        )

    failures = [r for r in results if not r.ok]
    durations = sorted(r.duration_sec for r in results)

    def percentile(values: List[float], p: float) -> float:
        if not values:
            return 0.0
        idx = int(round((len(values) - 1) * p))
        idx = max(0, min(len(values) - 1, idx))
        return values[idx]

    failures_count = len(failures)
    rounds_count = len(results)
    failure_rate = (100.0 * failures_count / rounds_count) if rounds_count else 0.0

    summary = {
        "rounds": args.rounds,
        "seed": args.seed,
        "ok": failures_count == 0,
        "failures": failures_count,
        "failure_rate_pct": failure_rate,
        "avg_duration_sec": (sum(r.duration_sec for r in results) / rounds_count)
        if rounds_count
        else 0.0,
        "min_duration_sec": durations[0] if durations else 0.0,
        "p50_duration_sec": percentile(durations, 0.50),
        "p95_duration_sec": percentile(durations, 0.95),
        "max_duration_sec": max((r.duration_sec for r in results), default=0.0),
    }
    payload = {
        "summary": summary,
        "results": [asdict(r) for r in results],
    }

    report_json = args.report_json if args.report_json.is_absolute() else root / args.report_json
    report_md = args.report_md if args.report_md.is_absolute() else root / args.report_md
    report_json.parent.mkdir(parents=True, exist_ok=True)
    report_md.parent.mkdir(parents=True, exist_ok=True)
    report_json.write_text(json.dumps(payload, indent=2), encoding="utf-8")

    md_lines = [
        "# Soak/Stress/Chaos Report",
        "",
        f"- rounds: `{summary['rounds']}`",
        f"- seed: `{summary['seed']}`",
        f"- ok: `{summary['ok']}`",
        f"- failures: `{summary['failures']}`",
        f"- failure_rate_pct: `{summary['failure_rate_pct']:.2f}`",
        f"- avg_duration_sec: `{summary['avg_duration_sec']:.3f}`",
        f"- min_duration_sec: `{summary['min_duration_sec']:.3f}`",
        f"- p50_duration_sec: `{summary['p50_duration_sec']:.3f}`",
        f"- p95_duration_sec: `{summary['p95_duration_sec']:.3f}`",
        f"- max_duration_sec: `{summary['max_duration_sec']:.3f}`",
        "",
    ]
    if failures:
        md_lines.append("## Failures")
        md_lines.append("")
        for fail in failures[:20]:
            md_lines.append(
                f"- round `{fail.round_index}` scenario `{fail.scenario}` rc `{fail.return_code}` chaos `{fail.chaos}`"
            )
    else:
        md_lines.append("All rounds passed.")

    report_md.write_text("\n".join(md_lines) + "\n", encoding="utf-8")
    print(f"soak/stress/chaos: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"json={report_json}")
    print(f"md={report_md}")
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
