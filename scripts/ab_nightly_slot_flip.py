#!/usr/bin/env python3
"""
Nightly A/B slot flip helper.

Chooses next slot (typically opposite of active) and stages/promotes fresh boot artifacts.
"""

from __future__ import annotations

import argparse
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def run(cmd: List[str], cwd: Path) -> tuple[bool, int, str, str]:
    proc = subprocess.run(
        cmd,
        cwd=str(cwd),
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode == 0, proc.returncode, proc.stdout, proc.stderr


def load_json(path: Path) -> Dict[str, Any]:
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description="Nightly A/B boot slot flip")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--ab-root",
        type=Path,
        default=Path("artifacts/boot_ab"),
        help="A/B state/artifact root",
    )
    parser.add_argument("--profile", choices=("debug", "release"), default="release")
    parser.add_argument("--target", default="x86_64-unknown-none")
    parser.add_argument("--force-slot", choices=("A", "B"), default=None)
    parser.add_argument("--version-tag", default=None)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--out-dir", type=Path, default=Path("reports/ab_slot_flip"))
    args = parser.parse_args()

    root = args.root.resolve()
    ab_root = args.ab_root if args.ab_root.is_absolute() else root / args.ab_root
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    state_path = ab_root / "state.json"

    init_cmd = ["python", "scripts/ab_boot_slots.py", "--ab-root", str(ab_root), "init"]
    ok, rc, out, err = run(init_cmd, root)
    if not ok:
        print(f"ab nightly slot flip: FAIL (init rc={rc})")
        return 1

    state = load_json(state_path)
    active_slot = str(state.get("active_slot", "A"))
    selected_slot = args.force_slot or ("B" if active_slot == "A" else "A")
    version_tag = args.version_tag or f"nightly-{args.profile}-{datetime.now(timezone.utc):%Y%m%d}"

    flip_cmd = [
        "python",
        "scripts/build_boot_image.py",
        "--target",
        args.target,
        "--profile",
        args.profile,
        "--ab-slot",
        selected_slot,
        "--ab-root",
        str(ab_root),
        "--ab-version",
        version_tag,
        "--ab-promote",
    ]

    build_ok = True
    build_rc = 0
    build_out = ""
    build_err = ""
    if not args.dry_run:
        build_ok, build_rc, build_out, build_err = run(flip_cmd, root)

    new_state = load_json(state_path)
    summary = {
        "ok": ok and build_ok,
        "dry_run": bool(args.dry_run),
        "active_slot_before": active_slot,
        "selected_slot": selected_slot,
        "active_slot_after": new_state.get("active_slot", active_slot),
        "pending_slot_after": new_state.get("pending_slot"),
        "version_tag": version_tag,
        "commands": {
            "init": init_cmd,
            "flip": flip_cmd,
        },
        "return_codes": {
            "init": rc,
            "flip": build_rc,
        },
        "state_path": str(state_path),
    }
    (out_dir / "summary.json").write_text(json.dumps({"summary": summary}, indent=2), encoding="utf-8")
    (out_dir / "summary.md").write_text(
        "\n".join(
            [
                "# A/B Nightly Slot Flip",
                "",
                f"- ok: `{summary['ok']}`",
                f"- dry_run: `{summary['dry_run']}`",
                f"- active_slot_before: `{summary['active_slot_before']}`",
                f"- selected_slot: `{summary['selected_slot']}`",
                f"- active_slot_after: `{summary['active_slot_after']}`",
                f"- pending_slot_after: `{summary['pending_slot_after']}`",
                f"- version_tag: `{summary['version_tag']}`",
                "",
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    print(f"ab nightly slot flip: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    if build_out:
        print(build_out.strip())
    if build_err:
        print(build_err.strip())
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
