#!/usr/bin/env python3
"""Validate HyperCore script localization key usage and locale parity."""

from __future__ import annotations

import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


WRITE_MSG_RE = re.compile(r'Write-HcMsg\s+"([^"]+)"')


@dataclass
class Report:
    generated_utc: str
    locale_paths: dict[str, str]
    usage_files: list[str]
    used_key_count: int
    missing_in_en: list[str]
    missing_in_tr: list[str]
    only_in_en: list[str]
    only_in_tr: list[str]
    ok: bool


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def collect_used_keys(files: list[Path]) -> set[str]:
    out: set[str] = set()
    for p in files:
        text = p.read_text(encoding="utf-8", errors="ignore")
        out.update(m.group(1) for m in WRITE_MSG_RE.finditer(text))
    return out


def main() -> int:
    root = Path(".").resolve()
    en_path = root / "scripts" / "i18n" / "en.json"
    tr_path = root / "scripts" / "i18n" / "tr.json"
    files = [
        root / "scripts" / "hypercore.ps1",
        root / "scripts" / "hypercore" / "novice.ps1",
        root / "scripts" / "hypercore" / "plugins.ps1",
    ]
    files = [p for p in files if p.exists()]

    en = load_json(en_path) if en_path.exists() else {}
    tr = load_json(tr_path) if tr_path.exists() else {}
    en_keys = set(en.keys())
    tr_keys = set(tr.keys())
    used = collect_used_keys(files)

    missing_in_en = sorted(k for k in used if k not in en_keys)
    missing_in_tr = sorted(k for k in used if k not in tr_keys)
    only_in_en = sorted(k for k in en_keys if k not in tr_keys)
    only_in_tr = sorted(k for k in tr_keys if k not in en_keys)
    ok = not (missing_in_en or missing_in_tr or only_in_en or only_in_tr)

    from datetime import datetime, timezone

    report = Report(
        generated_utc=datetime.now(timezone.utc).isoformat(),
        locale_paths={"en": str(en_path), "tr": str(tr_path)},
        usage_files=[str(p) for p in files],
        used_key_count=len(used),
        missing_in_en=missing_in_en,
        missing_in_tr=missing_in_tr,
        only_in_en=only_in_en,
        only_in_tr=only_in_tr,
        ok=ok,
    )

    out_path = root / "reports" / "tooling" / "script_i18n_lint.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(asdict(report), indent=2), encoding="utf-8")
    print(f"script i18n lint: {'PASS' if ok else 'FAIL'}")
    print(f"json={out_path}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())

