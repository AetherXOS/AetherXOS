#!/usr/bin/env python3
"""Run lightweight idempotency checks for tooling outputs."""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any


@dataclass
class Report:
    generated_utc: str
    ok: bool
    checks: dict[str, Any]
    issues: list[str]


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def norm_p_tier(obj: dict[str, Any]) -> dict[str, Any]:
    out = dict(obj)
    out.pop("generated_utc", None)
    hist = out.get("history")
    if isinstance(hist, dict):
        hist.pop("path", None)
        points = hist.get("points")
        if isinstance(points, list):
            for p in points:
                if isinstance(p, dict):
                    p.pop("generated_utc", None)
    return out


def run_p_tier_once(root: Path, out_json: Path, out_md: Path, history_json: Path) -> None:
    cmd = [
        sys.executable,
        str(root / "scripts" / "p_tier_status.py"),
        "--root",
        str(root),
        "--out-json",
        str(out_json),
        "--out-md",
        str(out_md),
        "--history-json",
        str(history_json),
        "--history-window",
        "2",
    ]
    subprocess.run(cmd, check=True, capture_output=True, text=True)


def main() -> int:
    root = Path(".").resolve()
    issues: list[str] = []
    checks: dict[str, Any] = {}

    with TemporaryDirectory(prefix="hc_idem_") as td:
        t = Path(td)
        out1 = t / "p1.json"
        md1 = t / "p1.md"
        hist1 = t / "h1.json"
        out2 = t / "p2.json"
        md2 = t / "p2.md"
        hist2 = t / "h2.json"

        try:
            run_p_tier_once(root, out1, md1, hist1)
            run_p_tier_once(root, out2, md2, hist2)
            j1 = norm_p_tier(load_json(out1))
            j2 = norm_p_tier(load_json(out2))
            same = j1 == j2
            checks["p_tier_status_deterministic"] = same
            if not same:
                issues.append("p_tier_status_changed_between_runs")
        except Exception as exc:
            checks["p_tier_status_deterministic"] = False
            issues.append(f"p_tier_status_exec_error:{exc}")

    try:
        commands = load_json(root / "scripts" / "config" / "hypercore.commands.json")
        canonical = json.dumps(commands, ensure_ascii=False, sort_keys=True)
        canonical_again = json.dumps(commands, ensure_ascii=False, sort_keys=True)
        same_cmd = canonical == canonical_again
        checks["command_catalog_stable"] = same_cmd
        if not same_cmd:
            issues.append("command_catalog_not_stable")
    except Exception as exc:
        checks["command_catalog_stable"] = False
        issues.append(f"command_catalog_error:{exc}")

    ok = len(issues) == 0 and all(bool(v) for v in checks.values())
    report = Report(generated_utc=now_utc(), ok=ok, checks=checks, issues=issues)
    out_path = root / "reports" / "tooling" / "idempotency_check.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(asdict(report), indent=2), encoding="utf-8")
    print(f"idempotency check: {'PASS' if ok else 'FAIL'}")
    print(f"json={out_path}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
