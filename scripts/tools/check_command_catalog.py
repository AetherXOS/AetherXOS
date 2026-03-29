#!/usr/bin/env python3
"""Validate scripts/config/hypercore.commands.json structure and consistency."""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path


@dataclass
class Report:
    generated_utc: str
    ok: bool
    issue_count: int
    issues: list[str]
    command_count: int
    unique_command_count: int


def now_utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def main() -> int:
    root = Path(".").resolve()
    path = root / "scripts" / "config" / "hypercore.commands.json"
    issues: list[str] = []
    commands = []
    if not path.exists():
        issues.append(f"missing:{path}")
    else:
        try:
            obj = json.loads(path.read_text(encoding="utf-8"))
            if not isinstance(obj, list):
                issues.append("root_not_array")
            else:
                commands = obj
        except Exception as exc:
            issues.append(f"invalid_json:{exc}")

    names: list[str] = []
    for i, c in enumerate(commands):
        if not isinstance(c, dict):
            issues.append(f"entry_not_object:{i}")
            continue
        for k in ("name", "desc_en", "desc_tr"):
            if k not in c or not isinstance(c[k], str) or not c[k].strip():
                issues.append(f"missing_or_invalid:{i}:{k}")
        name = str(c.get("name", "")).strip()
        if name:
            names.append(name)

    unique = set(names)
    if len(unique) != len(names):
        seen = set()
        for n in names:
            if n in seen:
                issues.append(f"duplicate_name:{n}")
            seen.add(n)

    out = Report(
        generated_utc=now_utc(),
        ok=len(issues) == 0,
        issue_count=len(issues),
        issues=issues,
        command_count=len(names),
        unique_command_count=len(unique),
    )
    out_path = root / "reports" / "tooling" / "command_catalog_validation.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(asdict(out), indent=2), encoding="utf-8")
    print(f"command catalog validation: {'PASS' if out.ok else 'FAIL'}")
    print(f"json={out_path}")
    return 0 if out.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
