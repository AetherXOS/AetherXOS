#!/usr/bin/env python3
"""
Generate TPM PCR/event-log summary report (best-effort).
"""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import datetime, timezone
from pathlib import Path


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description="Create PCR/event-log summary report")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--event-log", type=Path, default=Path("artifacts/tpm/eventlog.bin"))
    parser.add_argument("--out", type=Path, default=Path("reports/secureboot/pcr_report.json"))
    args = parser.parse_args()

    root = args.root.resolve()
    event_log = args.event_log if args.event_log.is_absolute() else root / args.event_log
    out_path = args.out if args.out.is_absolute() else root / args.out
    out_path.parent.mkdir(parents=True, exist_ok=True)

    exists = event_log.exists()
    payload = {
        "generated_utc": utc_now(),
        "ok": exists,
        "event_log_path": str(event_log),
        "event_log_exists": exists,
        "event_log_size_bytes": int(event_log.stat().st_size) if exists else 0,
        "event_log_sha256": sha256(event_log) if exists else "",
        "note": (
            "Best-effort local summary only. For full attestation parse TPM2 event format and verify PCR quotes."
        ),
    }
    out_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    print(f"secureboot-pcr-report: {'PASS' if payload['ok'] else 'WARN'}")
    print(f"report={out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

