#!/usr/bin/env python3
"""Generate aggregated P0/P1/P2 readiness status from existing gate reports."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def load_json(path: Path) -> Any | None:
    if not path.exists():
        return None
    try:
        raw = path.read_bytes()
        # Handle UTF-8/UTF-16 files written by mixed tooling.
        for enc in ("utf-8-sig", "utf-16", "utf-16-le", "utf-16-be", "utf-8"):
            try:
                return json.loads(raw.decode(enc))
            except Exception:
                continue
        return None
    except Exception:
        return None


def pick_latest(root: Path, patterns: Iterable[str]) -> Path | None:
    candidates: list[Path] = []
    for pattern in patterns:
        candidates.extend(root.glob(pattern))
    candidates = [p for p in candidates if p.is_file()]
    if not candidates:
        return None
    return sorted(candidates, key=lambda p: p.stat().st_mtime, reverse=True)[0]


@dataclass
class Check:
    id: str
    ok: bool
    required: bool
    detail: str
    path: str


def bool_check(root: Path, check_id: str, patterns: list[str], required: bool, predicate, missing_detail: str) -> Check:
    p = pick_latest(root, patterns)
    if p is None:
        return Check(check_id, False if required else True, required, missing_detail, "")
    data = load_json(p)
    if data is None:
        return Check(check_id, False if required else True, required, "invalid json", str(p))
    ok, detail = predicate(data)
    return Check(check_id, bool(ok), required, detail, str(p))


def summarize_tier(checks: list[Check]) -> dict[str, Any]:
    required_checks = [c for c in checks if c.required]
    passed_required = [c for c in required_checks if c.ok]
    ok = len(passed_required) == len(required_checks)
    score = 100.0 if not required_checks else round(100.0 * len(passed_required) / len(required_checks), 1)
    return {
        "ok": ok,
        "score_pct": score,
        "required_total": len(required_checks),
        "required_passed": len(passed_required),
        "required_remaining": max(0, len(required_checks) - len(passed_required)),
        "checks": [asdict(c) for c in checks],
    }


def hint_for_check(check_id: str) -> str:
    hints = {
        "health_score": "powershell -ExecutionPolicy Bypass -File .\\scripts\\hypercore.ps1 -Command health",
        "policy_gate": "powershell -ExecutionPolicy Bypass -File .\\scripts\\hypercore.ps1 -Command policy-gate",
        "syscall_default": "powershell -ExecutionPolicy Bypass -File .\\scripts\\release_preflight.ps1",
        "syscall_linux_compat": "powershell -ExecutionPolicy Bypass -File .\\scripts\\release_preflight.ps1",
        "reboot_recovery": "python scripts/reboot_recovery_gate.py --soak-summary artifacts/qemu_soak/summary.json",
        "p1_ops_gate": "python scripts/p1_ops_gate.py --skip-host-tests --soak-rounds 20 --soak-timeout-sec 300",
        "posix_conformance": "python scripts/posix_conformance_gate.py",
        "soak_stress_chaos": "python scripts/soak_stress_chaos.py --rounds 12",
        "qemu_soak": "python scripts/qemu_soak_matrix.py --profile release --rounds 10",
        "p2_gap_gate": "python scripts/p2_gap_gate.py --auto-baseline --update-baseline-on-success",
        "p2_actionable_markers": "python scripts/p2_gap_report.py",
        "release_candidate": "powershell -ExecutionPolicy Bypass -File .\\scripts\\release_candidate_gate.ps1",
    }
    return hints.get(check_id, "powershell -ExecutionPolicy Bypass -File .\\scripts\\hypercore.ps1 -Command help")


def build_trend(current: dict[str, Any], previous: dict[str, Any] | None) -> dict[str, Any]:
    out: dict[str, Any] = {"overall_regression": False, "tiers": {}}
    if not previous:
        for tier_name in ("p0", "p1", "p2"):
            out["tiers"][tier_name] = {
                "prev_score_pct": None,
                "delta_score_pct": 0.0,
                "regression": False,
            }
        return out
    for tier_name in ("p0", "p1", "p2"):
        cur = float(current.get("tiers", {}).get(tier_name, {}).get("score_pct", 0))
        prev = float(previous.get("tiers", {}).get(tier_name, {}).get("score_pct", 0))
        delta = round(cur - prev, 1)
        regression = delta < 0
        out["tiers"][tier_name] = {
            "prev_score_pct": prev,
            "delta_score_pct": delta,
            "regression": regression,
        }
        if regression:
            out["overall_regression"] = True
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate P0/P1/P2 tier readiness status")
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument("--out-json", type=Path, default=Path("reports/tooling/p_tier_status.json"))
    parser.add_argument("--out-md", type=Path, default=Path("reports/tooling/p_tier_status.md"))
    parser.add_argument("--history-json", type=Path, default=Path("reports/tooling/p_tier_status_history.json"))
    parser.add_argument("--history-window", type=int, default=60)
    args = parser.parse_args()

    root = args.root.resolve()

    p0_checks = [
        bool_check(
            root,
            "health_score",
            ["reports/tooling/health_report.json"],
            True,
            lambda d: ((d.get("score", 0) >= 60), f"score={d.get('score', 0)}"),
            "missing health_report",
        ),
        bool_check(
            root,
            "policy_gate",
            ["reports/tooling/policy_gate.json"],
            True,
            lambda d: (bool(d.get("ok", False)), f"ok={bool(d.get('ok', False))}"),
            "missing policy gate",
        ),
        bool_check(
            root,
            "syscall_default",
            ["reports/syscall_coverage_summary.json"],
            True,
            lambda d: ((float(d.get("implemented_pct", 0)) >= 100.0), f"implemented_pct={d.get('implemented_pct', 0)}"),
            "missing syscall coverage summary",
        ),
        bool_check(
            root,
            "syscall_linux_compat",
            ["reports/syscall_coverage_linux_compat_summary.json"],
            True,
            lambda d: ((float(d.get("implemented_pct", 0)) >= 100.0), f"implemented_pct={d.get('implemented_pct', 0)}"),
            "missing linux_compat syscall coverage summary",
        ),
        bool_check(
            root,
            "reboot_recovery",
            ["reports/reboot_recovery_gate*/summary.json"],
            False,
            lambda d: (bool(d.get("ok", False)), f"ok={bool(d.get('ok', False))}"),
            "no reboot recovery summary",
        ),
    ]

    p1_checks = [
        bool_check(
            root,
            "p1_ops_gate",
            ["reports/p1_ops_gate/summary.json", "reports/p1_nightly/summary.json"],
            True,
            lambda d: (bool(d.get("summary", {}).get("ok", d.get("ok", False))), "summary.ok"),
            "missing p1 summary",
        ),
        bool_check(
            root,
            "posix_conformance",
            ["reports/posix_conformance/summary.json"],
            True,
            lambda d: (bool(d.get("summary", {}).get("ok", False)), "summary.ok"),
            "missing posix conformance summary",
        ),
        bool_check(
            root,
            "soak_stress_chaos",
            ["reports/soak_stress_chaos.json"],
            True,
            lambda d: (bool(d.get("summary", {}).get("ok", False)), "summary.ok"),
            "missing soak/stress summary",
        ),
        bool_check(
            root,
            "qemu_soak",
            ["artifacts/qemu_soak/summary.json"],
            False,
            lambda d: (bool(d.get("ok", False)) and not bool(d.get("dry_run", False)), f"ok={d.get('ok', False)} dry_run={d.get('dry_run', False)}"),
            "missing qemu soak summary",
        ),
    ]

    p2_checks = [
        bool_check(
            root,
            "p2_gap_gate",
            ["reports/p2_gap/gate_summary.json"],
            True,
            lambda d: (bool(d.get("summary", {}).get("ok", False)), "summary.ok"),
            "missing p2 gap gate summary",
        ),
        bool_check(
            root,
            "p2_actionable_markers",
            ["reports/p2_gap/gate_summary.json"],
            True,
            lambda d: (
                int(d.get("summary", {}).get("current", {}).get("actionable_total_markers", 9999)) <= 0,
                f"actionable_total_markers={d.get('summary', {}).get('current', {}).get('actionable_total_markers', 'n/a')}",
            ),
            "missing p2 gap marker data",
        ),
        bool_check(
            root,
            "release_candidate",
            ["reports/release_candidate*/verdict.json"],
            False,
            lambda d: (bool(d.get("ok", False)), f"ok={bool(d.get('ok', False))} ready={bool(d.get('ready', False))}"),
            "missing release candidate verdict",
        ),
    ]

    tiers = {
        "p0": summarize_tier(p0_checks),
        "p1": summarize_tier(p1_checks),
        "p2": summarize_tier(p2_checks),
    }
    overall_ok = tiers["p0"]["ok"] and tiers["p1"]["ok"] and tiers["p2"]["ok"]
    blockers: list[str] = []
    blocker_items: list[dict[str, Any]] = []
    remaining_required: list[dict[str, Any]] = []
    remaining_optional: list[dict[str, Any]] = []
    for tier_name, tier in tiers.items():
        for c in tier["checks"]:
            if c["required"] and not c["ok"]:
                blockers.append(f"{tier_name}:{c['id']} ({c['detail']})")
            if not c["ok"]:
                item = {
                    "tier": tier_name,
                    "id": c["id"],
                    "required": bool(c["required"]),
                    "detail": c["detail"],
                    "path": c["path"],
                    "hint_command": hint_for_check(c["id"]),
                }
                if c["required"]:
                    blocker_items.append(item)
                    remaining_required.append(item)
                else:
                    remaining_optional.append(item)

    previous_status = load_json(args.out_json)
    current_core = {"tiers": tiers}
    trend = build_trend(current_core, previous_status if isinstance(previous_status, dict) else None)

    history_obj = load_json(args.history_json)
    history_points = []
    if isinstance(history_obj, dict) and isinstance(history_obj.get("points"), list):
        history_points = list(history_obj["points"])
    history_points.append(
        {
            "generated_utc": utc_now(),
            "overall_ok": overall_ok,
            "p0_score_pct": tiers["p0"]["score_pct"],
            "p1_score_pct": tiers["p1"]["score_pct"],
            "p2_score_pct": tiers["p2"]["score_pct"],
        }
    )
    history_points = history_points[-max(1, int(args.history_window)) :]
    history_out = {
        "generated_utc": utc_now(),
        "window": int(args.history_window),
        "points": history_points,
    }

    required_total = sum(int(t["required_total"]) for t in tiers.values())
    required_passed = sum(int(t["required_passed"]) for t in tiers.values())
    overall_completion_pct = 100.0 if required_total <= 0 else round((100.0 * required_passed) / required_total, 1)

    out = {
        "generated_utc": utc_now(),
        "overall_ok": overall_ok,
        "overall_completion_pct": overall_completion_pct,
        "required_total": required_total,
        "required_passed": required_passed,
        "required_remaining": max(0, required_total - required_passed),
        "blockers": blockers,
        "blocker_items": blocker_items,
        "remaining": {
            "required": remaining_required,
            "optional": remaining_optional,
            "required_count": len(remaining_required),
            "optional_count": len(remaining_optional),
        },
        "trend": trend,
        "history": {
            "path": str(args.history_json.resolve()),
            "points": history_points,
        },
        "tiers": tiers,
    }

    args.out_json.parent.mkdir(parents=True, exist_ok=True)
    args.out_json.write_text(json.dumps(out, indent=2), encoding="utf-8")
    args.history_json.parent.mkdir(parents=True, exist_ok=True)
    args.history_json.write_text(json.dumps(history_out, indent=2), encoding="utf-8")

    md_lines = [
        "# P0/P1/P2 Tier Status",
        "",
        f"- generated_utc: `{out['generated_utc']}`",
        f"- overall_ok: `{out['overall_ok']}`",
        f"- overall_completion_pct: `{out['overall_completion_pct']}`",
        f"- required_passed: `{out['required_passed']}/{out['required_total']}`",
        f"- blockers: `{len(blockers)}`",
        "",
    ]
    for tier_name in ("p0", "p1", "p2"):
        t = tiers[tier_name]
        md_lines.append(f"## {tier_name.upper()} - {'OK' if t['ok'] else 'FAIL'}")
        md_lines.append(f"- score_pct: `{t['score_pct']}`")
        md_lines.append(f"- required_passed: `{t['required_passed']}/{t['required_total']}`")
        td = out["trend"]["tiers"][tier_name]
        md_lines.append(f"- trend_delta_score_pct: `{td['delta_score_pct']}`")
        md_lines.append("")
        for c in t["checks"]:
            req = "required" if c["required"] else "optional"
            md_lines.append(f"- [{ 'x' if c['ok'] else ' ' }] `{c['id']}` ({req}) - {c['detail']}")
        md_lines.append("")

    if blockers:
        md_lines.append("## Blockers")
        for b in blocker_items:
            md_lines.append(f"- {b['tier']}:{b['id']} ({b['detail']})")
            md_lines.append(f"  - hint: `{b['hint_command']}`")
        md_lines.append("")

    if remaining_optional:
        md_lines.append("## Optional Remaining")
        for b in remaining_optional:
            md_lines.append(f"- {b['tier']}:{b['id']} ({b['detail']})")
            md_lines.append(f"  - hint: `{b['hint_command']}`")
        md_lines.append("")

    args.out_md.parent.mkdir(parents=True, exist_ok=True)
    args.out_md.write_text("\n".join(md_lines), encoding="utf-8")

    print(f"p tier status: {'PASS' if overall_ok else 'FAIL'}")
    print(f"json={args.out_json.resolve()}")
    print(f"md={args.out_md.resolve()}")
    return 0 if overall_ok else 0


if __name__ == "__main__":
    raise SystemExit(main())
