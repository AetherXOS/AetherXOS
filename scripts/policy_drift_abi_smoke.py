#!/usr/bin/env python3
"""
Policy-drift control ABI smoke test.

Host-side checker that validates:
1) Core syscall numbers for policy-drift control endpoints are present/stable.
2) Generated governor constants for policy-drift sample/cooldown are present.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path


NR_RE = re.compile(r"pub const ([A-Z0-9_]+): usize = (\d+);")
CONST_U64_RE = re.compile(r"pub const ([A-Z0-9_]+): u64 = (\d+);")


def parse_nr_map(path: Path) -> dict[str, int]:
    text = path.read_text(encoding="utf-8", errors="replace")
    return {name: int(value) for name, value in NR_RE.findall(text)}


def parse_u64_consts(path: Path) -> dict[str, int]:
    text = path.read_text(encoding="utf-8", errors="replace")
    return {name: int(value) for name, value in CONST_U64_RE.findall(text)}


def main() -> int:
    parser = argparse.ArgumentParser(description="Policy drift ABI smoke checker")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON output")
    args = parser.parse_args()

    nr_map = parse_nr_map(args.root / "src/kernel/syscalls/syscalls_consts.rs")
    generated = parse_u64_consts(args.root / "src/generated_consts.rs")

    expected_nr = {
        "SET_POLICY_DRIFT_CONTROL": 58,
        "GET_POLICY_DRIFT_CONTROL": 59,
        "GET_POLICY_DRIFT_REASON_TEXT": 60,
    }

    required_consts = [
        "GOVERNOR_RUNTIME_POLICY_DRIFT_SAMPLE_INTERVAL_TICKS",
        "GOVERNOR_RUNTIME_POLICY_DRIFT_REAPPLY_COOLDOWN_TICKS",
    ]

    failures: list[str] = []

    for name, expected in expected_nr.items():
        got = nr_map.get(name)
        if got != expected:
            failures.append(f"{name}: expected {expected}, got {got}")

    for name in required_consts:
        value = generated.get(name)
        if value is None or value <= 0:
            failures.append(f"{name}: missing or invalid ({value})")

    out = {
        "ok": len(failures) == 0,
        "failures": failures,
        "syscalls": {k: nr_map.get(k) for k in expected_nr},
        "consts": {k: generated.get(k) for k in required_consts},
    }

    if args.json:
        print(json.dumps(out, indent=2))
    else:
        if out["ok"]:
            print("policy-drift ABI smoke: PASS")
            for k, v in out["syscalls"].items():
                print(f"  {k}={v}")
            for k, v in out["consts"].items():
                print(f"  {k}={v}")
        else:
            print("policy-drift ABI smoke: FAIL")
            for f in failures:
                print(f"  - {f}")

    return 0 if out["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
