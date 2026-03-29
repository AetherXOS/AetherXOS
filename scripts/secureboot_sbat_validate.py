#!/usr/bin/env python3
"""
Basic SBAT presence validator for EFI binaries.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import List


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def has_sbat_blob(path: Path) -> bool:
    data = path.read_bytes()
    return b"sbat," in data or b".sbat" in data


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate SBAT metadata presence in EFI binaries")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--efi-dir", type=Path, default=Path("artifacts/boot_image/iso_root/EFI/BOOT"))
    parser.add_argument("--report", type=Path, default=Path("reports/secureboot/sbat_report.json"))
    parser.add_argument("--required", default="shimx64.efi,grubx64.efi")
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Return non-zero when missing files or SBAT markers are detected",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    efi_dir = args.efi_dir if args.efi_dir.is_absolute() else root / args.efi_dir
    report_path = args.report if args.report.is_absolute() else root / args.report
    required = [x.strip() for x in args.required.split(",") if x.strip()]

    failures: List[str] = []
    rows = []
    for name in required:
        p = efi_dir / name
        if not p.exists():
            failures.append(f"missing required EFI: {name}")
            rows.append({"file": name, "exists": False, "has_sbat": False})
            continue
        has_sbat = has_sbat_blob(p)
        rows.append({"file": name, "exists": True, "has_sbat": has_sbat})
        if not has_sbat:
            failures.append(f"sbat marker missing: {name}")

    ok = len(failures) == 0
    status = "PASS" if ok else ("FAIL" if args.strict else "WARN")
    summary = {
        "generated_utc": utc_now(),
        "ok": ok,
        "status": status,
        "strict": bool(args.strict),
        "efi_dir": str(efi_dir),
        "rows": rows,
        "failures": failures,
        "note": "Heuristic scan; for strict verification use objdump/readelf tooling in CI.",
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(f"secureboot-sbat: {status}")
    print(f"report={report_path}")
    if ok:
        return 0
    return 1 if args.strict else 0


if __name__ == "__main__":
    raise SystemExit(main())
