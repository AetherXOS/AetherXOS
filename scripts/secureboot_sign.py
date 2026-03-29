#!/usr/bin/env python3
"""
Sign EFI binaries for Secure Boot workflows (sbsign/pesign).
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import List


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def run(cmd: List[str], cwd: Path) -> int:
    proc = subprocess.run(cmd, cwd=str(cwd), check=False)
    return int(proc.returncode)


def list_targets(efi_dir: Path) -> List[Path]:
    preferred = [
        efi_dir / "BOOTX64.EFI",
        efi_dir / "shimx64.efi",
        efi_dir / "grubx64.efi",
        efi_dir / "liminex64.efi",
        efi_dir / "mmx64.efi",
        efi_dir / "fbx64.efi",
    ]
    out: List[Path] = []
    for p in preferred:
        if p.exists() and p not in out:
            out.append(p)
    for p in sorted(efi_dir.glob("*.efi")):
        if p not in out:
            out.append(p)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Sign EFI binaries for Secure Boot")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--efi-dir", type=Path, default=Path("artifacts/boot_image/iso_root/EFI/BOOT"))
    parser.add_argument("--out-dir", type=Path, default=Path("artifacts/secureboot/signed"))
    parser.add_argument("--tool", choices=("auto", "sbsign", "pesign"), default="auto")
    parser.add_argument("--key", type=Path, default=Path("keys/db.key"))
    parser.add_argument("--cert", type=Path, default=Path("keys/db.crt"))
    parser.add_argument("--pesign-cert", default="")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--report", type=Path, default=Path("reports/secureboot/sign_report.json"))
    args = parser.parse_args()

    root = args.root.resolve()
    efi_dir = args.efi_dir if args.efi_dir.is_absolute() else root / args.efi_dir
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    report_path = args.report if args.report.is_absolute() else root / args.report
    key = args.key if args.key.is_absolute() else root / args.key
    cert = args.cert if args.cert.is_absolute() else root / args.cert

    if not efi_dir.exists():
        print(f"secureboot-sign: FAIL (missing efi dir: {efi_dir})")
        return 1

    tool = args.tool
    if tool == "auto":
        if shutil.which("sbsign"):
            tool = "sbsign"
        elif shutil.which("pesign"):
            tool = "pesign"
        else:
            tool = "none"

    targets = list_targets(efi_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    failures: List[str] = []
    rows = []

    for src in targets:
        dst = out_dir / src.name
        if args.dry_run or tool == "none":
            shutil.copy2(src, dst)
            rows.append({"file": src.name, "signed": False, "tool": tool, "out": str(dst), "dry_run": True})
            continue

        if tool == "sbsign":
            if not key.exists() or not cert.exists():
                failures.append(f"missing key/cert for sbsign: key={key} cert={cert}")
                continue
            cmd = [
                "sbsign",
                "--key",
                str(key),
                "--cert",
                str(cert),
                "--output",
                str(dst),
                str(src),
            ]
            rc = run(cmd, root)
            ok = rc == 0
            rows.append({"file": src.name, "signed": ok, "tool": "sbsign", "out": str(dst), "rc": rc})
            if not ok:
                failures.append(f"sbsign failed: {src.name} rc={rc}")
        elif tool == "pesign":
            if not args.pesign_cert:
                failures.append("pesign requires --pesign-cert")
                continue
            cmd = [
                "pesign",
                "--force",
                "--sign",
                "--certificate",
                args.pesign_cert,
                "--in",
                str(src),
                "--out",
                str(dst),
            ]
            rc = run(cmd, root)
            ok = rc == 0
            rows.append({"file": src.name, "signed": ok, "tool": "pesign", "out": str(dst), "rc": rc})
            if not ok:
                failures.append(f"pesign failed: {src.name} rc={rc}")

    summary = {
        "generated_utc": utc_now(),
        "ok": len(failures) == 0,
        "tool": tool,
        "targets": len(targets),
        "rows": rows,
        "failures": failures,
        "efi_dir": str(efi_dir),
        "out_dir": str(out_dir),
        "note": "Dry-run/none tool mode copies files without signing.",
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(f"secureboot-sign: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"report={report_path}")
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())

