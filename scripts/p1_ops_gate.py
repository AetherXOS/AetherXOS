#!/usr/bin/env python3
"""
P1 operations gate for HyperCore.

Pipeline:
1) Run release preflight.
2) Run host soak/stress/chaos rounds.
3) Optionally run QEMU soak matrix + reboot recovery gate.
4) Emit a single aggregated summary for CI/manual review.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Tuple


def run_cmd(cmd: List[str], cwd: Path) -> Tuple[bool, int, str, str]:
    proc = subprocess.run(
        cmd,
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode == 0, proc.returncode, proc.stdout, proc.stderr


def load_json(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def tail(text: str, lines: int = 40) -> str:
    return "\n".join(text.splitlines()[-lines:])


def read_optional_json(path: Path) -> Dict[str, Any]:
    if path.exists():
        return load_json(path)
    return {}


def read_optional_summary(path: Path) -> Dict[str, Any]:
    payload = read_optional_json(path)
    if not payload:
        return {}
    summary = payload.get("summary")
    if isinstance(summary, dict):
        return summary
    return payload


def read_optional_list_json(path: Path) -> List[Dict[str, Any]]:
    payload = read_optional_json(path)
    if not payload:
        return []
    if isinstance(payload, list):
        return payload
    if isinstance(payload, dict) and isinstance(payload.get("runs"), list):
        return payload["runs"]
    return []


def summarize_recent_runs(history: List[Dict[str, Any]], window: int) -> Dict[str, Any]:
    if window <= 0:
        return {"window": 0, "count": 0}
    recent = history[-window:]
    if not recent:
        return {"window": window, "count": 0}
    soak = [r.get("soak_summary", {}) for r in recent]
    avg_values = [float(s.get("avg_duration_sec", 0.0)) for s in soak]
    p95_values = [float(s.get("p95_duration_sec", 0.0)) for s in soak]
    fail_rate_values = [float(s.get("failure_rate_pct", 0.0)) for s in soak]
    return {
        "window": window,
        "count": len(recent),
        "avg_duration_sec_mean": sum(avg_values) / len(avg_values),
        "p95_duration_sec_mean": sum(p95_values) / len(p95_values),
        "failure_rate_pct_mean": sum(fail_rate_values) / len(fail_rate_values),
        "ok_ratio": (sum(1 for r in recent if bool(r.get("ok", False))) / len(recent)),
    }


def compare_against_trend(
    current_soak: Dict[str, Any],
    trend: Dict[str, Any],
    *,
    max_avg_vs_trend_increase_pct: float,
    max_p95_vs_trend_increase_pct: float,
    max_failure_rate_vs_trend_increase_pctpoint: float,
) -> List[str]:
    failures: List[str] = []
    count = int(trend.get("count", 0))
    if count <= 0:
        return failures

    def pct_increase(cur: float, base: float) -> float:
        if base <= 0.0:
            return 0.0 if cur <= 0.0 else 100.0
        return ((cur - base) / base) * 100.0

    cur_avg = float(current_soak.get("avg_duration_sec", 0.0))
    trend_avg = float(trend.get("avg_duration_sec_mean", 0.0))
    avg_inc = pct_increase(cur_avg, trend_avg)
    if avg_inc > max_avg_vs_trend_increase_pct:
        failures.append(
            f"avg_duration trend regression: +{avg_inc:.2f}% > max +{max_avg_vs_trend_increase_pct:.2f}%"
        )

    cur_p95 = float(current_soak.get("p95_duration_sec", 0.0))
    trend_p95 = float(trend.get("p95_duration_sec_mean", 0.0))
    p95_inc = pct_increase(cur_p95, trend_p95)
    if p95_inc > max_p95_vs_trend_increase_pct:
        failures.append(
            f"p95_duration trend regression: +{p95_inc:.2f}% > max +{max_p95_vs_trend_increase_pct:.2f}%"
        )

    cur_fail_rate = float(current_soak.get("failure_rate_pct", 0.0))
    trend_fail_rate = float(trend.get("failure_rate_pct_mean", 0.0))
    fail_rate_inc = cur_fail_rate - trend_fail_rate
    if fail_rate_inc > max_failure_rate_vs_trend_increase_pctpoint:
        failures.append(
            f"failure_rate trend regression: +{fail_rate_inc:.2f}pp > max +{max_failure_rate_vs_trend_increase_pctpoint:.2f}pp"
        )

    return failures


def validate_syscall_summary(
    summary: Dict[str, Any],
    *,
    min_implemented_pct: float,
    max_no: int,
    max_partial: int,
    max_external: int,
) -> List[str]:
    failures: List[str] = []
    if not summary:
        return ["missing syscall summary report"]
    implemented_pct = float(summary.get("implemented_pct", 0.0))
    no_count = int(summary.get("no", 10**9))
    partial_count = int(summary.get("partial", 10**9))
    external_count = int(summary.get("external", 10**9))

    if implemented_pct < min_implemented_pct:
        failures.append(
            f"implemented_pct {implemented_pct:.2f} < min {min_implemented_pct:.2f}"
        )
    if no_count > max_no:
        failures.append(f"no {no_count} > max_no {max_no}")
    if partial_count > max_partial:
        failures.append(f"partial {partial_count} > max_partial {max_partial}")
    if external_count > max_external:
        failures.append(f"external {external_count} > max_external {max_external}")
    return failures


def compare_against_baseline(
    current: Dict[str, Any],
    baseline: Dict[str, Any],
    *,
    max_failure_increase: int,
    max_failure_rate_increase_pctpoint: float,
    max_avg_duration_increase_pct: float,
    max_p95_duration_increase_pct: float,
    max_max_duration_increase_pct: float,
) -> List[str]:
    failures: List[str] = []
    if not baseline:
        return failures

    cur_fail = int(current.get("failures", 0))
    base_fail = int(baseline.get("failures", 0))
    if cur_fail > base_fail + max_failure_increase:
        failures.append(
            f"soak failures regression: current {cur_fail} > baseline {base_fail} + {max_failure_increase}"
        )
    cur_fail_rate = float(current.get("failure_rate_pct", 0.0))
    base_fail_rate = float(baseline.get("failure_rate_pct", 0.0))
    fail_rate_increase = cur_fail_rate - base_fail_rate
    if fail_rate_increase > max_failure_rate_increase_pctpoint:
        failures.append(
            f"failure_rate regression: +{fail_rate_increase:.2f}pp > max +{max_failure_rate_increase_pctpoint:.2f}pp"
        )

    def pct_increase(cur: float, base: float) -> float:
        if base <= 0.0:
            return 0.0 if cur <= 0.0 else 100.0
        return ((cur - base) / base) * 100.0

    cur_avg = float(current.get("avg_duration_sec", 0.0))
    base_avg = float(baseline.get("avg_duration_sec", 0.0))
    avg_inc = pct_increase(cur_avg, base_avg)
    if avg_inc > max_avg_duration_increase_pct:
        failures.append(
            f"avg_duration_sec regression: +{avg_inc:.2f}% > max +{max_avg_duration_increase_pct:.2f}%"
        )

    cur_max = float(current.get("max_duration_sec", 0.0))
    base_max = float(baseline.get("max_duration_sec", 0.0))
    max_inc = pct_increase(cur_max, base_max)
    if max_inc > max_max_duration_increase_pct:
        failures.append(
            f"max_duration_sec regression: +{max_inc:.2f}% > max +{max_max_duration_increase_pct:.2f}%"
        )

    cur_p95 = float(current.get("p95_duration_sec", cur_max))
    base_p95 = float(baseline.get("p95_duration_sec", base_max))
    p95_inc = pct_increase(cur_p95, base_p95)
    if p95_inc > max_p95_duration_increase_pct:
        failures.append(
            f"p95_duration_sec regression: +{p95_inc:.2f}% > max +{max_p95_duration_increase_pct:.2f}%"
        )

    return failures


def compare_qemu_against_baseline(
    current: Dict[str, Any],
    baseline: Dict[str, Any],
    *,
    max_failed_rounds_increase: int,
    max_expected_success_drop: int,
) -> List[str]:
    failures: List[str] = []
    if not baseline:
        return failures

    cur_failed = int(current.get("failed_rounds", 0))
    base_failed = int(baseline.get("failed_rounds", 0))
    if cur_failed > base_failed + max_failed_rounds_increase:
        failures.append(
            f"qemu failed_rounds regression: current {cur_failed} > baseline {base_failed} + {max_failed_rounds_increase}"
        )

    cur_expected = int(current.get("expected_success_rounds", 0))
    base_expected = int(baseline.get("expected_success_rounds", 0))
    if cur_expected < base_expected - max_expected_success_drop:
        failures.append(
            f"qemu expected_success_rounds regression: current {cur_expected} < baseline {base_expected} - {max_expected_success_drop}"
        )

    return failures


def compare_reboot_against_baseline(
    current: Dict[str, Any],
    baseline: Dict[str, Any],
    *,
    max_failures_increase: int,
    max_successful_rounds_drop: int,
) -> List[str]:
    failures: List[str] = []
    if not baseline:
        return failures

    cur_fail = len(current.get("failures", []))
    base_fail = len(baseline.get("failures", []))
    if cur_fail > base_fail + max_failures_increase:
        failures.append(
            f"reboot failures regression: current {cur_fail} > baseline {base_fail} + {max_failures_increase}"
        )

    cur_success = int(current.get("successful_rounds", 0))
    base_success = int(baseline.get("successful_rounds", 0))
    if cur_success < base_success - max_successful_rounds_drop:
        failures.append(
            f"reboot successful_rounds regression: current {cur_success} < baseline {base_success} - {max_successful_rounds_drop}"
        )

    if baseline.get("crash_pipeline_ok", True) and not current.get("crash_pipeline_ok", False):
        failures.append("reboot crash_pipeline_ok regressed to false")

    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description="Run P1 ops gate")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument("--skip-host-tests", action="store_true")
    parser.add_argument("--soak-rounds", type=int, default=20)
    parser.add_argument("--soak-timeout-sec", type=int, default=300)
    parser.add_argument("--soak-seed", type=int, default=20260305)
    parser.add_argument("--max-soak-failures", type=int, default=0)
    parser.add_argument(
        "--baseline-json",
        type=Path,
        default=None,
        help="Optional previous P1 summary JSON for regression comparison",
    )
    parser.add_argument(
        "--write-baseline-json",
        type=Path,
        default=None,
        help="Optional path to write current soak summary as baseline",
    )
    parser.add_argument("--max-failure-increase", type=int, default=0)
    parser.add_argument("--max-failure-rate-increase-pctpoint", type=float, default=0.0)
    parser.add_argument("--max-avg-duration-increase-pct", type=float, default=25.0)
    parser.add_argument("--max-p95-duration-increase-pct", type=float, default=35.0)
    parser.add_argument("--max-max-duration-increase-pct", type=float, default=40.0)
    parser.add_argument(
        "--auto-baseline",
        action="store_true",
        help="If no baseline is provided, use <out-dir>/baseline_soak_summary.json when present",
    )
    parser.add_argument(
        "--update-baseline-on-success",
        action="store_true",
        help="Write current soak summary to baseline file when gate succeeds",
    )
    parser.add_argument("--default-min-implemented-pct", type=float, default=100.0)
    parser.add_argument("--default-max-no", type=int, default=0)
    parser.add_argument("--default-max-partial", type=int, default=0)
    parser.add_argument("--default-max-external", type=int, default=0)
    parser.add_argument("--linux-min-implemented-pct", type=float, default=100.0)
    parser.add_argument("--linux-max-no", type=int, default=0)
    parser.add_argument("--linux-max-partial", type=int, default=0)
    parser.add_argument("--linux-max-external", type=int, default=0)
    parser.add_argument("--run-qemu-soak", action="store_true")
    parser.add_argument("--qemu-dry-run", action="store_true")
    parser.add_argument("--qemu-boot-mode", choices=("direct", "iso"), default="direct")
    parser.add_argument("--qemu-iso-path", type=Path, default=None)
    parser.add_argument("--qemu-build-iso", action="store_true")
    parser.add_argument("--qemu-limine-bin-dir", type=Path, default=None)
    parser.add_argument("--qemu-iso-name", default="hypercore.iso")
    parser.add_argument("--qemu-auto-fetch-limine", action="store_true")
    parser.add_argument("--qemu-limine-version", default="latest")
    parser.add_argument("--qemu-limine-cache-dir", type=Path, default=Path("artifacts/limine/cache"))
    parser.add_argument("--qemu-limine-out-dir", type=Path, default=Path("artifacts/limine/bin"))
    parser.add_argument("--qemu-allow-build-limine", action="store_true")
    parser.add_argument("--qemu-allow-timeout-success", action="store_true")
    parser.add_argument("--qemu-rounds", type=int, default=10)
    parser.add_argument("--qemu-memory-mb", default="512,1024")
    parser.add_argument("--qemu-cores", default="1,2")
    parser.add_argument("--qemu-chaos-rate", type=float, default=0.35)
    parser.add_argument("--qemu-timeout-sec", type=int, default=45)
    parser.add_argument("--max-qemu-failed-rounds-increase", type=int, default=0)
    parser.add_argument("--max-qemu-expected-success-drop", type=int, default=0)
    parser.add_argument("--max-reboot-failures-increase", type=int, default=0)
    parser.add_argument("--max-reboot-successful-rounds-drop", type=int, default=0)
    parser.add_argument(
        "--qemu-baseline-json",
        type=Path,
        default=None,
        help="Optional baseline JSON for qemu soak summary regression check",
    )
    parser.add_argument(
        "--reboot-baseline-json",
        type=Path,
        default=None,
        help="Optional baseline JSON for reboot recovery summary regression check",
    )
    parser.add_argument("--auto-qemu-baseline", action="store_true")
    parser.add_argument("--auto-reboot-baseline", action="store_true")
    parser.add_argument("--update-qemu-baseline-on-success", action="store_true")
    parser.add_argument("--update-reboot-baseline-on-success", action="store_true")
    parser.add_argument(
        "--run-ab-recovery-gate",
        action="store_true",
        help="Run A/B recovery gate using reboot recovery summary",
    )
    parser.add_argument(
        "--require-ab-recovery-gate",
        action="store_true",
        help="Fail if A/B recovery gate is requested but cannot be executed",
    )
    parser.add_argument(
        "--require-ab-pending-cleared",
        action="store_true",
        help="Fail if A/B state still has pending_slot after recovery gate",
    )
    parser.add_argument(
        "--ab-state-json",
        type=Path,
        default=Path("artifacts/boot_ab/state.json"),
        help="Path to A/B state json used by ab_boot_recovery_gate",
    )
    parser.add_argument(
        "--require-qemu-baseline",
        action="store_true",
        help="Fail if qemu baseline is missing/empty when qemu soak is enabled",
    )
    parser.add_argument(
        "--require-reboot-baseline",
        action="store_true",
        help="Fail if reboot baseline is missing/empty when qemu soak is enabled",
    )
    parser.add_argument(
        "--history-json",
        type=Path,
        default=None,
        help="Optional run history JSON path. Defaults to <out-dir>/history.json",
    )
    parser.add_argument("--history-retention", type=int, default=200)
    parser.add_argument("--trend-window", type=int, default=10)
    parser.add_argument("--enforce-trend", action="store_true")
    parser.add_argument("--max-avg-vs-trend-increase-pct", type=float, default=35.0)
    parser.add_argument("--max-p95-vs-trend-increase-pct", type=float, default=45.0)
    parser.add_argument(
        "--max-failure-rate-vs-trend-increase-pctpoint", type=float, default=0.0
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("reports/p1_ops_gate"),
        help="Output directory for summary artifacts",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    failures: List[str] = []
    steps: List[Dict[str, Any]] = []

    preflight_cmd = [
        "powershell",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        ".\\scripts\\release_preflight.ps1",
    ]
    if args.skip_host_tests:
        preflight_cmd.append("-SkipHostTests")
    ok, rc, out, err = run_cmd(preflight_cmd, root)
    steps.append(
        {
            "name": "release_preflight",
            "ok": ok,
            "return_code": rc,
            "stdout_tail": tail(out),
            "stderr_tail": tail(err),
        }
    )
    if not ok:
        failures.append("release_preflight failed")

    default_cov_path = root / "reports" / "syscall_coverage_summary.json"
    linux_cov_path = root / "reports" / "syscall_coverage_linux_compat_summary.json"
    default_cov = read_optional_json(default_cov_path)
    linux_cov = read_optional_json(linux_cov_path)
    default_cov_failures = validate_syscall_summary(
        default_cov,
        min_implemented_pct=args.default_min_implemented_pct,
        max_no=args.default_max_no,
        max_partial=args.default_max_partial,
        max_external=args.default_max_external,
    )
    linux_cov_failures = validate_syscall_summary(
        linux_cov,
        min_implemented_pct=args.linux_min_implemented_pct,
        max_no=args.linux_max_no,
        max_partial=args.linux_max_partial,
        max_external=args.linux_max_external,
    )
    if default_cov_failures:
        failures.extend([f"default syscall coverage: {msg}" for msg in default_cov_failures])
    if linux_cov_failures:
        failures.extend([f"linux_compat syscall coverage: {msg}" for msg in linux_cov_failures])
    steps.append(
        {
            "name": "syscall_coverage_reports",
            "ok": len(default_cov_failures) == 0 and len(linux_cov_failures) == 0,
            "return_code": 0,
            "default_summary": default_cov,
            "linux_compat_summary": linux_cov,
            "default_failures": default_cov_failures,
            "linux_failures": linux_cov_failures,
        }
    )

    soak_json = root / "reports" / "soak_stress_chaos.json"
    soak_md = root / "reports" / "soak_stress_chaos.md"
    soak_cmd = [
        "python",
        "scripts/soak_stress_chaos.py",
        "--rounds",
        str(args.soak_rounds),
        "--timeout-sec",
        str(args.soak_timeout_sec),
        "--seed",
        str(args.soak_seed),
        "--report-json",
        str(soak_json),
        "--report-md",
        str(soak_md),
    ]
    ok, rc, out, err = run_cmd(soak_cmd, root)
    soak_summary: Dict[str, Any] = {}
    if soak_json.exists():
        soak_payload = load_json(soak_json)
        soak_summary = soak_payload.get("summary", {})
        soak_failures = int(soak_summary.get("failures", 0))
        if soak_failures > args.max_soak_failures:
            failures.append(
                f"soak failures {soak_failures} > max_soak_failures {args.max_soak_failures}"
            )
    elif not ok:
        failures.append("soak_stress_chaos failed and no report produced")

    steps.append(
        {
            "name": "soak_stress_chaos",
            "ok": ok,
            "return_code": rc,
            "summary": soak_summary,
            "stdout_tail": tail(out),
            "stderr_tail": tail(err),
        }
    )

    posix_cmd = [
        "python",
        "scripts/posix_conformance_gate.py",
    ]
    ok, rc, out, err = run_cmd(posix_cmd, root)
    posix_summary_path = root / "reports" / "posix_conformance" / "summary.json"
    posix_summary: Dict[str, Any] = {}
    if posix_summary_path.exists():
        posix_payload = load_json(posix_summary_path)
        posix_summary = posix_payload.get("summary", {})
        if not bool(posix_summary.get("ok", False)):
            failures.append("posix conformance gate summary reported failure")
    elif not ok:
        failures.append("posix conformance gate failed and no report produced")
    if not ok:
        failures.append("posix conformance gate command failed")

    steps.append(
        {
            "name": "posix_conformance_gate",
            "ok": ok,
            "return_code": rc,
            "summary": posix_summary,
            "stdout_tail": tail(out),
            "stderr_tail": tail(err),
        }
    )

    baseline_path = None
    baseline_summary: Dict[str, Any] = {}
    auto_baseline_path = out_dir / "baseline_soak_summary.json"
    if args.baseline_json is not None:
        baseline_path = args.baseline_json if args.baseline_json.is_absolute() else root / args.baseline_json
    elif args.auto_baseline and auto_baseline_path.exists():
        baseline_path = auto_baseline_path

    if baseline_path is not None:
        baseline_payload = read_optional_json(baseline_path)
        baseline_summary = baseline_payload.get("summary", baseline_payload)
        regression_failures = compare_against_baseline(
            soak_summary,
            baseline_summary,
            max_failure_increase=args.max_failure_increase,
            max_failure_rate_increase_pctpoint=args.max_failure_rate_increase_pctpoint,
            max_avg_duration_increase_pct=args.max_avg_duration_increase_pct,
            max_p95_duration_increase_pct=args.max_p95_duration_increase_pct,
            max_max_duration_increase_pct=args.max_max_duration_increase_pct,
        )
        if regression_failures:
            failures.extend([f"baseline regression: {msg}" for msg in regression_failures])
        steps.append(
            {
                "name": "baseline_regression_check",
                "ok": len(regression_failures) == 0,
                "return_code": 0,
                "baseline_path": str(baseline_path),
                "baseline_summary": baseline_summary,
                "failures": regression_failures,
            }
        )

    if args.update_baseline_on_success and len(failures) == 0:
        auto_baseline_path.parent.mkdir(parents=True, exist_ok=True)
        auto_baseline_path.write_text(
            json.dumps({"summary": soak_summary}, indent=2), encoding="utf-8"
        )
        steps.append(
            {
                "name": "update_baseline_on_success",
                "ok": True,
                "return_code": 0,
                "path": str(auto_baseline_path),
            }
        )

    if args.write_baseline_json is not None:
        write_path = args.write_baseline_json
        if not write_path.is_absolute():
            write_path = root / write_path
        write_path.parent.mkdir(parents=True, exist_ok=True)
        write_path.write_text(json.dumps({"summary": soak_summary}, indent=2), encoding="utf-8")
        steps.append(
            {
                "name": "write_baseline",
                "ok": True,
                "return_code": 0,
                "path": str(write_path),
            }
        )

    reboot_summary: Dict[str, Any] = {}
    qemu_summary: Dict[str, Any] = {}
    ab_recovery_summary: Dict[str, Any] = {}
    ab_gate_executed = False
    if args.run_qemu_soak:
        qemu_cmd = [
            "python",
            "scripts/qemu_soak_matrix.py",
            "--profile",
            "release",
            "--boot-mode",
            args.qemu_boot_mode,
            "--rounds",
            str(args.qemu_rounds),
            "--memory-mb",
            args.qemu_memory_mb,
            "--cores",
            args.qemu_cores,
            "--chaos-rate",
            str(args.qemu_chaos_rate),
            "--round-timeout-sec",
            str(args.qemu_timeout_sec),
            "--out-dir",
            "artifacts/qemu_soak",
        ]
        if args.qemu_iso_path is not None:
            iso_path = args.qemu_iso_path if args.qemu_iso_path.is_absolute() else root / args.qemu_iso_path
            qemu_cmd.extend(["--iso-path", str(iso_path)])
        if args.qemu_build_iso:
            qemu_cmd.append("--build-iso")
            if args.qemu_limine_bin_dir is not None:
                limine_dir = (
                    args.qemu_limine_bin_dir
                    if args.qemu_limine_bin_dir.is_absolute()
                    else root / args.qemu_limine_bin_dir
                )
                qemu_cmd.extend(["--limine-bin-dir", str(limine_dir)])
            qemu_cmd.extend(["--iso-name", args.qemu_iso_name])
        if args.qemu_auto_fetch_limine:
            qemu_cmd.append("--auto-fetch-limine")
            qemu_cmd.extend(["--limine-version", args.qemu_limine_version])
            limine_cache_dir = (
                args.qemu_limine_cache_dir
                if args.qemu_limine_cache_dir.is_absolute()
                else root / args.qemu_limine_cache_dir
            )
            limine_out_dir = (
                args.qemu_limine_out_dir
                if args.qemu_limine_out_dir.is_absolute()
                else root / args.qemu_limine_out_dir
            )
            qemu_cmd.extend(["--limine-cache-dir", str(limine_cache_dir)])
            qemu_cmd.extend(["--limine-out-dir", str(limine_out_dir)])
            if args.qemu_allow_build_limine:
                qemu_cmd.append("--allow-build-limine")
        if args.qemu_allow_timeout_success:
            qemu_cmd.append("--allow-timeout-success")
        if args.qemu_dry_run:
            qemu_cmd.append("--dry-run")
        ok, rc, out, err = run_cmd(qemu_cmd, root)
        qemu_summary_path = root / "artifacts" / "qemu_soak" / "summary.json"
        if qemu_summary_path.exists():
            qemu_summary = read_optional_summary(qemu_summary_path)
        steps.append(
            {
                "name": "qemu_soak_matrix",
                "ok": ok,
                "return_code": rc,
                "summary": qemu_summary,
                "stdout_tail": tail(out),
                "stderr_tail": tail(err),
            }
        )
        if not ok:
            failures.append("qemu_soak_matrix failed")

        if args.qemu_dry_run:
            steps.append(
                {
                    "name": "reboot_recovery_gate",
                    "ok": True,
                    "return_code": 0,
                    "summary": {},
                    "stdout_tail": "skipped (qemu dry-run)",
                    "stderr_tail": "",
                }
            )
        else:
            recovery_cmd = [
                "python",
                "scripts/reboot_recovery_gate.py",
                "--soak-summary",
                "artifacts/qemu_soak/summary.json",
                "--out-dir",
                "reports/reboot_recovery_gate",
            ]
            ok, rc, out, err = run_cmd(recovery_cmd, root)
            recovery_summary_path = root / "reports" / "reboot_recovery_gate" / "summary.json"
            if recovery_summary_path.exists():
                reboot_summary = read_optional_summary(recovery_summary_path)
            steps.append(
                {
                    "name": "reboot_recovery_gate",
                    "ok": ok,
                    "return_code": rc,
                    "summary": reboot_summary,
                    "stdout_tail": tail(out),
                    "stderr_tail": tail(err),
                }
            )
            if not ok:
                failures.append("reboot_recovery_gate failed")

            if args.run_ab_recovery_gate and recovery_summary_path.exists():
                ab_cmd = [
                    "python",
                    "scripts/ab_boot_recovery_gate.py",
                    "--reboot-summary",
                    str(recovery_summary_path),
                    "--ab-state",
                    str(args.ab_state_json),
                ]
                ok, rc, out, err = run_cmd(ab_cmd, root)
                ab_gate_executed = True
                ab_summary_path = root / "reports" / "ab_boot_recovery_gate" / "summary.json"
                if ab_summary_path.exists():
                    ab_recovery_summary = read_optional_summary(ab_summary_path)
                steps.append(
                    {
                        "name": "ab_boot_recovery_gate",
                        "ok": ok,
                        "return_code": rc,
                        "summary": ab_recovery_summary,
                        "stdout_tail": tail(out),
                        "stderr_tail": tail(err),
                    }
                )
                if not ok:
                    failures.append("ab_boot_recovery_gate failed")
                elif args.require_ab_pending_cleared:
                    if ab_recovery_summary.get("pending_slot") is not None:
                        failures.append("ab_boot_recovery_gate did not clear pending_slot")
                        steps.append(
                            {
                                "name": "ab_pending_clear_check",
                                "ok": False,
                                "return_code": 1,
                                "pending_slot": ab_recovery_summary.get("pending_slot"),
                            }
                        )
                    else:
                        steps.append(
                            {
                                "name": "ab_pending_clear_check",
                                "ok": True,
                                "return_code": 0,
                            }
                        )

    if args.run_ab_recovery_gate and args.require_ab_recovery_gate:
        if not args.run_qemu_soak:
            failures.append("ab recovery gate required but qemu soak is disabled")
            steps.append(
                {
                    "name": "ab_recovery_gate_required_check",
                    "ok": False,
                    "return_code": 1,
                    "reason": "run_qemu_soak disabled",
                }
            )
        elif args.qemu_dry_run:
            failures.append("ab recovery gate required but qemu dry-run is enabled")
            steps.append(
                {
                    "name": "ab_recovery_gate_required_check",
                    "ok": False,
                    "return_code": 1,
                    "reason": "qemu dry-run",
                }
            )
        elif not ab_gate_executed:
            failures.append("ab recovery gate required but not executed")
            steps.append(
                {
                    "name": "ab_recovery_gate_required_check",
                    "ok": False,
                    "return_code": 1,
                    "reason": "missing reboot summary or gate skipped",
                }
            )
        else:
            steps.append(
                {
                    "name": "ab_recovery_gate_required_check",
                    "ok": True,
                    "return_code": 0,
                }
            )

    qemu_baseline_path = None
    reboot_baseline_path = None
    qemu_baseline_summary: Dict[str, Any] = {}
    reboot_baseline_summary: Dict[str, Any] = {}
    auto_qemu_baseline_path = out_dir / "baseline_qemu_soak_summary.json"
    auto_reboot_baseline_path = out_dir / "baseline_reboot_recovery_summary.json"

    if args.qemu_baseline_json is not None:
        qemu_baseline_path = (
            args.qemu_baseline_json
            if args.qemu_baseline_json.is_absolute()
            else root / args.qemu_baseline_json
        )
    elif args.auto_qemu_baseline and auto_qemu_baseline_path.exists():
        qemu_baseline_path = auto_qemu_baseline_path

    if args.reboot_baseline_json is not None:
        reboot_baseline_path = (
            args.reboot_baseline_json
            if args.reboot_baseline_json.is_absolute()
            else root / args.reboot_baseline_json
        )
    elif args.auto_reboot_baseline and auto_reboot_baseline_path.exists():
        reboot_baseline_path = auto_reboot_baseline_path

    if args.run_qemu_soak and not args.qemu_dry_run and args.require_qemu_baseline:
        if qemu_baseline_path is None:
            failures.append("qemu baseline required but missing")
            steps.append(
                {
                    "name": "qemu_baseline_required_check",
                    "ok": False,
                    "return_code": 1,
                    "reason": "missing baseline path",
                }
            )
        else:
            qemu_baseline_summary = read_optional_summary(qemu_baseline_path)
            baseline_has_rounds = (
                isinstance(qemu_baseline_summary, dict)
                and int(qemu_baseline_summary.get("rounds", 0)) > 0
            )
            if not baseline_has_rounds:
                failures.append("qemu baseline required but empty/invalid")
            steps.append(
                {
                    "name": "qemu_baseline_required_check",
                    "ok": baseline_has_rounds,
                    "return_code": 0 if baseline_has_rounds else 1,
                    "baseline_path": str(qemu_baseline_path),
                }
            )

    if args.run_qemu_soak and not args.qemu_dry_run and args.require_reboot_baseline:
        if reboot_baseline_path is None:
            failures.append("reboot baseline required but missing")
            steps.append(
                {
                    "name": "reboot_baseline_required_check",
                    "ok": False,
                    "return_code": 1,
                    "reason": "missing baseline path",
                }
            )
        else:
            reboot_baseline_summary = read_optional_summary(reboot_baseline_path)
            baseline_has_rounds = (
                isinstance(reboot_baseline_summary, dict)
                and int(reboot_baseline_summary.get("total_rounds", 0)) > 0
            )
            if not baseline_has_rounds:
                failures.append("reboot baseline required but empty/invalid")
            steps.append(
                {
                    "name": "reboot_baseline_required_check",
                    "ok": baseline_has_rounds,
                    "return_code": 0 if baseline_has_rounds else 1,
                    "baseline_path": str(reboot_baseline_path),
                }
            )

    if qemu_baseline_path is not None and qemu_summary:
        qemu_baseline_summary = read_optional_summary(qemu_baseline_path)
        qemu_regressions = compare_qemu_against_baseline(
            qemu_summary,
            qemu_baseline_summary,
            max_failed_rounds_increase=args.max_qemu_failed_rounds_increase,
            max_expected_success_drop=args.max_qemu_expected_success_drop,
        )
        if qemu_regressions:
            failures.extend([f"qemu baseline regression: {msg}" for msg in qemu_regressions])
        steps.append(
            {
                "name": "qemu_baseline_regression_check",
                "ok": len(qemu_regressions) == 0,
                "return_code": 0,
                "baseline_path": str(qemu_baseline_path),
                "baseline_summary": qemu_baseline_summary,
                "failures": qemu_regressions,
            }
        )

    if reboot_baseline_path is not None and reboot_summary:
        reboot_baseline_summary = read_optional_summary(reboot_baseline_path)
        reboot_regressions = compare_reboot_against_baseline(
            reboot_summary,
            reboot_baseline_summary,
            max_failures_increase=args.max_reboot_failures_increase,
            max_successful_rounds_drop=args.max_reboot_successful_rounds_drop,
        )
        if reboot_regressions:
            failures.extend([f"reboot baseline regression: {msg}" for msg in reboot_regressions])
        steps.append(
            {
                "name": "reboot_baseline_regression_check",
                "ok": len(reboot_regressions) == 0,
                "return_code": 0,
                "baseline_path": str(reboot_baseline_path),
                "baseline_summary": reboot_baseline_summary,
                "failures": reboot_regressions,
            }
        )

    if args.update_qemu_baseline_on_success and len(failures) == 0 and qemu_summary:
        auto_qemu_baseline_path.parent.mkdir(parents=True, exist_ok=True)
        auto_qemu_baseline_path.write_text(
            json.dumps({"summary": qemu_summary}, indent=2), encoding="utf-8"
        )
        steps.append(
            {
                "name": "update_qemu_baseline_on_success",
                "ok": True,
                "return_code": 0,
                "path": str(auto_qemu_baseline_path),
            }
        )

    if args.update_reboot_baseline_on_success and len(failures) == 0 and reboot_summary:
        auto_reboot_baseline_path.parent.mkdir(parents=True, exist_ok=True)
        auto_reboot_baseline_path.write_text(
            json.dumps({"summary": reboot_summary}, indent=2), encoding="utf-8"
        )
        steps.append(
            {
                "name": "update_reboot_baseline_on_success",
                "ok": True,
                "return_code": 0,
                "path": str(auto_reboot_baseline_path),
            }
        )

    history_path = (
        args.history_json
        if args.history_json is not None
        else (out_dir / "history.json")
    )
    if not history_path.is_absolute():
        history_path = root / history_path
    history = read_optional_list_json(history_path)
    trend_summary = summarize_recent_runs(history, args.trend_window)
    trend_failures: List[str] = []
    if args.enforce_trend:
        trend_failures = compare_against_trend(
            soak_summary,
            trend_summary,
            max_avg_vs_trend_increase_pct=args.max_avg_vs_trend_increase_pct,
            max_p95_vs_trend_increase_pct=args.max_p95_vs_trend_increase_pct,
            max_failure_rate_vs_trend_increase_pctpoint=args.max_failure_rate_vs_trend_increase_pctpoint,
        )
        if trend_failures:
            failures.extend([f"trend regression: {msg}" for msg in trend_failures])

    current_run_record = {
        "timestamp_utc": datetime.now(timezone.utc).isoformat(),
        "ok": len(failures) == 0,
        "failure_count": len(failures),
        "failures": list(failures),
        "soak_summary": soak_summary,
        "qemu_soak_summary": qemu_summary,
        "reboot_recovery_summary": reboot_summary,
        "ab_boot_recovery_summary": ab_recovery_summary,
    }
    history.append(current_run_record)
    if args.history_retention > 0 and len(history) > args.history_retention:
        history = history[-args.history_retention :]
    history_path.parent.mkdir(parents=True, exist_ok=True)
    history_path.write_text(json.dumps({"runs": history}, indent=2), encoding="utf-8")
    steps.append(
        {
            "name": "history_update",
            "ok": True,
            "return_code": 0,
            "path": str(history_path),
            "retention": args.history_retention,
            "count": len(history),
        }
    )
    steps.append(
        {
            "name": "trend_check",
            "ok": len(trend_failures) == 0,
            "return_code": 0,
            "enforced": bool(args.enforce_trend),
            "trend_summary": trend_summary,
            "failures": trend_failures,
        }
    )

    summary = {
        "ok": len(failures) == 0,
        "failures": failures,
        "steps": [{"name": s["name"], "ok": s["ok"], "return_code": s["return_code"]} for s in steps],
        "soak_summary": soak_summary,
        "baseline_summary": baseline_summary,
        "qemu_soak_summary": qemu_summary,
        "qemu_baseline_summary": qemu_baseline_summary,
        "reboot_recovery_summary": reboot_summary,
        "reboot_baseline_summary": reboot_baseline_summary,
        "ab_boot_recovery_summary": ab_recovery_summary,
        "trend_summary": trend_summary,
    }
    payload = {"summary": summary, "steps": steps}

    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    md = [
        "# P1 Ops Gate",
        "",
        f"- ok: `{summary['ok']}`",
        "",
        "## Steps",
        "",
    ]
    for s in summary["steps"]:
        md.append(f"- `{s['name']}` ok=`{s['ok']}` rc=`{s['return_code']}`")
    if failures:
        md.extend(["", "## Failures", ""])
        for f in failures:
            md.append(f"- {f}")
    (out_dir / "summary.md").write_text("\n".join(md) + "\n", encoding="utf-8")

    print(f"p1 ops gate: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
