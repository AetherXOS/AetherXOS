#!/usr/bin/env python3
"""
Manage A/B boot slots for HyperCore boot artifacts.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while True:
            chunk = f.read(1024 * 1024)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def default_state() -> Dict[str, Any]:
    return {
        "active_slot": "A",
        "last_known_good_slot": "A",
        "previous_slot": None,
        "pending_slot": None,
        "status": "healthy",
        "policy": {
            "max_consecutive_failures": 3
        },
        "slots": {
            "A": {"generation": 0, "version": None, "updated_at_utc": None, "artifacts": {}, "boot_failures": 0, "boot_successes": 0},
            "B": {"generation": 0, "version": None, "updated_at_utc": None, "artifacts": {}, "boot_failures": 0, "boot_successes": 0},
        },
        "history": [],
    }


def load_state(path: Path) -> Dict[str, Any]:
    if not path.exists():
        return default_state()
    payload = json.loads(path.read_text(encoding="utf-8"))
    base = default_state()
    base.update(payload)
    return base


def save_state(path: Path, state: Dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(state, indent=2), encoding="utf-8")


def append_history(state: Dict[str, Any], event: str, details: Dict[str, Any]) -> None:
    history = state.setdefault("history", [])
    history.append({"ts_utc": utc_now(), "event": event, "details": details})
    if len(history) > 200:
        state["history"] = history[-200:]


def main() -> int:
    parser = argparse.ArgumentParser(description="Manage HyperCore A/B boot slots")
    parser.add_argument(
        "--ab-root",
        type=Path,
        default=Path("artifacts/boot_ab"),
        help="A/B metadata/artifact root directory",
    )
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("init", help="Initialize A/B state")

    stage = sub.add_parser("stage", help="Stage boot artifacts into a slot")
    stage.add_argument("--slot", choices=("A", "B"), required=True)
    stage.add_argument("--kernel", type=Path, required=True)
    stage.add_argument("--initramfs", type=Path, required=True)
    stage.add_argument("--limine-cfg", type=Path, required=True)
    stage.add_argument("--version", default=None)
    stage.add_argument("--promote", action="store_true")

    mark_good = sub.add_parser("mark-good", help="Mark a slot as known-good")
    mark_good.add_argument("--slot", choices=("A", "B"), default=None)

    rollback = sub.add_parser("rollback", help="Rollback active slot")
    rollback.add_argument("--to-slot", choices=("A", "B"), default=None)
    rollback.add_argument("--reason", default="manual rollback")

    sub.add_parser("status", help="Print current slot state")

    boot_report = sub.add_parser("boot-report", help="Record boot outcome and enforce failure rollback policy")
    boot_report.add_argument("--slot", choices=("A", "B"), default=None)
    boot_report.add_argument("--result", choices=("ok", "fail"), required=True)
    boot_report.add_argument("--reason", default="")
    boot_report.add_argument("--max-failures", type=int, default=0)

    args = parser.parse_args()
    ab_root = args.ab_root
    if not ab_root.is_absolute():
        ab_root = Path.cwd() / ab_root
    state_path = ab_root / "state.json"

    state = load_state(state_path)
    cmd = args.cmd

    if cmd == "init":
        save_state(state_path, state)
        print(f"ab slots initialized: {state_path}")
        return 0

    if cmd == "stage":
        slot = args.slot
        slot_boot_dir = ab_root / "slots" / slot / "boot"
        slot_boot_dir.mkdir(parents=True, exist_ok=True)
        kernel_dst = slot_boot_dir / "hypercore.elf"
        initrd_dst = slot_boot_dir / "initramfs.cpio.gz"
        limine_dst = slot_boot_dir / "limine.cfg"
        shutil.copy2(args.kernel, kernel_dst)
        shutil.copy2(args.initramfs, initrd_dst)
        shutil.copy2(args.limine_cfg, limine_dst)

        slot_meta = state.setdefault("slots", {}).setdefault(slot, {})
        slot_meta["generation"] = int(slot_meta.get("generation", 0)) + 1
        slot_meta["version"] = args.version
        slot_meta["updated_at_utc"] = utc_now()
        slot_meta["artifacts"] = {
            "kernel_path": str(kernel_dst),
            "initramfs_path": str(initrd_dst),
            "limine_cfg_path": str(limine_dst),
            "kernel_sha256": sha256_file(kernel_dst),
            "initramfs_sha256": sha256_file(initrd_dst),
            "limine_cfg_sha256": sha256_file(limine_dst),
        }
        append_history(
            state,
            "stage",
            {"slot": slot, "generation": slot_meta["generation"], "version": args.version},
        )

        if args.promote:
            prev = state.get("active_slot")
            if prev != slot:
                state["previous_slot"] = prev
            state["active_slot"] = slot
            state["pending_slot"] = slot
            state["status"] = "pending_validation"
            append_history(
                state,
                "promote_pending",
                {"slot": slot, "previous_slot": prev},
            )

        save_state(state_path, state)
        print(f"slot staged: slot={slot} state={state_path}")
        return 0

    if cmd == "mark-good":
        slot = args.slot or state.get("active_slot", "A")
        state["last_known_good_slot"] = slot
        state["pending_slot"] = None
        state["status"] = "healthy"
        append_history(state, "mark_good", {"slot": slot})
        save_state(state_path, state)
        print(f"slot marked good: slot={slot}")
        return 0

    if cmd == "rollback":
        to_slot = args.to_slot or state.get("last_known_good_slot") or "A"
        prev = state.get("active_slot")
        state["previous_slot"] = prev
        state["active_slot"] = to_slot
        state["pending_slot"] = None
        state["status"] = "rolled_back"
        append_history(
            state,
            "rollback",
            {"from_slot": prev, "to_slot": to_slot, "reason": args.reason},
        )
        save_state(state_path, state)
        print(f"rollback complete: {prev} -> {to_slot}")
        return 0

    if cmd == "status":
        print(json.dumps(state, indent=2))
        return 0

    if cmd == "boot-report":
        slot = args.slot or state.get("active_slot", "A")
        slot_meta = state.setdefault("slots", {}).setdefault(slot, {})
        slot_meta["boot_failures"] = int(slot_meta.get("boot_failures", 0))
        slot_meta["boot_successes"] = int(slot_meta.get("boot_successes", 0))

        policy = state.setdefault("policy", {})
        current_policy = int(policy.get("max_consecutive_failures", 3))
        max_failures = int(args.max_failures) if int(args.max_failures) > 0 else current_policy
        policy["max_consecutive_failures"] = max_failures

        rolled_back = False
        from_slot = state.get("active_slot")
        to_slot = None
        if args.result == "ok":
            slot_meta["boot_successes"] += 1
            slot_meta["boot_failures"] = 0
            if state.get("pending_slot") == slot:
                state["last_known_good_slot"] = slot
                state["pending_slot"] = None
                state["status"] = "healthy"
        else:
            slot_meta["boot_failures"] += 1
            if slot_meta["boot_failures"] >= max_failures:
                to_slot = state.get("last_known_good_slot") or ("B" if slot == "A" else "A")
                if to_slot != slot:
                    state["previous_slot"] = slot
                    state["active_slot"] = to_slot
                    state["pending_slot"] = None
                    state["status"] = "rolled_back"
                    rolled_back = True

        append_history(
            state,
            "boot_report",
            {
                "slot": slot,
                "result": args.result,
                "reason": args.reason,
                "boot_failures": slot_meta["boot_failures"],
                "boot_successes": slot_meta["boot_successes"],
                "max_consecutive_failures": max_failures,
                "rolled_back": rolled_back,
                "from_slot": from_slot,
                "to_slot": to_slot,
            },
        )
        save_state(state_path, state)
        print(
            json.dumps(
                {
                    "ok": True,
                    "slot": slot,
                    "result": args.result,
                    "boot_failures": slot_meta["boot_failures"],
                    "boot_successes": slot_meta["boot_successes"],
                    "rolled_back": rolled_back,
                    "active_slot": state.get("active_slot"),
                    "last_known_good_slot": state.get("last_known_good_slot"),
                },
                indent=2,
            )
        )
        return 0

    raise RuntimeError(f"unsupported command: {cmd}")


if __name__ == "__main__":
    raise SystemExit(main())
