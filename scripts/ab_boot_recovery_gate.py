#!/usr/bin/env python3
"""
A/B boot recovery gate.

Consumes reboot recovery summary and updates A/B slot state:
- On success: pending slot becomes known-good.
- On failure: rollback to last known-good slot.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def load_json(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def save_json(path: Path, payload: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def append_history(state: Dict[str, Any], event: str, details: Dict[str, Any]) -> None:
    history = state.setdefault("history", [])
    history.append({"ts_utc": utc_now(), "event": event, "details": details})
    if len(history) > 200:
        state["history"] = history[-200:]


def main() -> int:
    parser = argparse.ArgumentParser(description="Apply reboot results to A/B slot state")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--ab-state",
        type=Path,
        default=Path("artifacts/boot_ab/state.json"),
        help="A/B state json path",
    )
    parser.add_argument(
        "--reboot-summary",
        type=Path,
        default=Path("reports/reboot_recovery_gate/summary.json"),
        help="Reboot recovery summary json path",
    )
    parser.add_argument("--min-successful-rounds", type=int, default=1)
    parser.add_argument("--out-dir", type=Path, default=Path("reports/ab_boot_recovery_gate"))
    args = parser.parse_args()

    root = args.root.resolve()
    state_path = args.ab_state if args.ab_state.is_absolute() else root / args.ab_state
    reboot_path = args.reboot_summary if args.reboot_summary.is_absolute() else root / args.reboot_summary
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    failures: List[str] = []
    actions: List[str] = []

    if not state_path.exists():
        failures.append(f"missing ab state: {state_path}")
    if not reboot_path.exists():
        failures.append(f"missing reboot summary: {reboot_path}")
    if failures:
        summary = {"ok": False, "failures": failures, "actions": actions}
        save_json(out_dir / "summary.json", summary)
        print("ab boot recovery gate: FAIL")
        return 1

    state = load_json(state_path)
    reboot = load_json(reboot_path)

    reboot_ok = bool(reboot.get("ok", False))
    successful_rounds = int(reboot.get("successful_rounds", 0))
    pending_slot = state.get("pending_slot")
    active_slot = state.get("active_slot")
    lkg_slot = state.get("last_known_good_slot") or active_slot

    if pending_slot is None:
        actions.append("no pending slot; no state transition")
    else:
        if reboot_ok and successful_rounds >= args.min_successful_rounds:
            state["last_known_good_slot"] = pending_slot
            state["pending_slot"] = None
            state["status"] = "healthy"
            append_history(
                state,
                "auto_mark_good",
                {"slot": pending_slot, "successful_rounds": successful_rounds},
            )
            actions.append(f"marked pending slot good: {pending_slot}")
        else:
            state["previous_slot"] = active_slot
            state["active_slot"] = lkg_slot
            state["pending_slot"] = None
            state["status"] = "rolled_back"
            append_history(
                state,
                "auto_rollback",
                {
                    "from_slot": active_slot,
                    "to_slot": lkg_slot,
                    "reboot_ok": reboot_ok,
                    "successful_rounds": successful_rounds,
                },
            )
            actions.append(f"rolled back: {active_slot} -> {lkg_slot}")

    save_json(state_path, state)
    summary = {
        "ok": len(failures) == 0,
        "failures": failures,
        "actions": actions,
        "reboot_ok": reboot_ok,
        "successful_rounds": successful_rounds,
        "active_slot": state.get("active_slot"),
        "last_known_good_slot": state.get("last_known_good_slot"),
        "pending_slot": state.get("pending_slot"),
        "state_path": str(state_path),
    }
    save_json(out_dir / "summary.json", summary)
    md_lines = [
        "# A/B Boot Recovery Gate",
        "",
        f"- ok: `{summary['ok']}`",
        f"- reboot_ok: `{summary['reboot_ok']}`",
        f"- successful_rounds: `{summary['successful_rounds']}`",
        f"- active_slot: `{summary['active_slot']}`",
        f"- last_known_good_slot: `{summary['last_known_good_slot']}`",
        f"- pending_slot: `{summary['pending_slot']}`",
        "",
        "## Actions",
        "",
    ]
    for item in actions:
        md_lines.append(f"- {item}")
    if failures:
        md_lines.extend(["", "## Failures", ""])
        for item in failures:
            md_lines.append(f"- {item}")
    (out_dir / "summary.md").write_text("\n".join(md_lines) + "\n", encoding="utf-8")

    print(f"ab boot recovery gate: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
