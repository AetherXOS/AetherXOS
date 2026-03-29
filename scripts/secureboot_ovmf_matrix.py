#!/usr/bin/env python3
"""
Run basic OVMF Secure Boot matrix smoke scenarios in QEMU.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import time
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import List, Optional


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def find_qemu() -> Optional[str]:
    candidates = ["qemu-system-x86_64"]
    if os.name == "nt":
        pf = os.environ.get("ProgramFiles", r"C:\Program Files")
        candidates.append(str(Path(pf) / "qemu" / "qemu-system-x86_64.exe"))
    for c in candidates:
        if shutil.which(c) or Path(c).exists():
            return c
    return None


@dataclass
class CaseResult:
    name: str
    secure_boot: bool
    ok: bool
    rc: int
    timeout: bool
    duration_sec: float
    log_path: str


def run_case(
    qemu: str,
    iso: Path,
    ovmf_code: Path,
    ovmf_vars_template: Path,
    secure_boot: bool,
    out_dir: Path,
    timeout_sec: int,
) -> CaseResult:
    case_name = "secure_on" if secure_boot else "secure_off"
    vars_copy = out_dir / f"OVMF_VARS_{case_name}.fd"
    shutil.copy2(ovmf_vars_template, vars_copy)
    log_path = out_dir / f"{case_name}.log"
    args = [
        qemu,
        "-nographic",
        "-m",
        "1024",
        "-smp",
        "2",
        "-drive",
        f"if=pflash,format=raw,readonly=on,file={ovmf_code}",
        "-drive",
        f"if=pflash,format=raw,file={vars_copy}",
        "-cdrom",
        str(iso),
        "-boot",
        "d",
    ]
    start = time.perf_counter()
    proc = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    timeout = False
    out = ""
    try:
        out, _ = proc.communicate(timeout=timeout_sec)
    except subprocess.TimeoutExpired:
        timeout = True
        proc.terminate()
        out, _ = proc.communicate(timeout=5)
    duration = time.perf_counter() - start
    log_path.write_text(out or "", encoding="utf-8", errors="replace")
    rc = proc.returncode if proc.returncode is not None else -1
    ok = (rc == 0) and (not timeout)
    return CaseResult(case_name, secure_boot, ok, rc, timeout, duration, str(log_path))


def main() -> int:
    parser = argparse.ArgumentParser(description="Run OVMF secure boot matrix smoke")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--iso", type=Path, default=Path("artifacts/boot_image/hypercore.iso"))
    parser.add_argument("--ovmf-code", type=Path, default=Path("artifacts/ovmf/OVMF_CODE.fd"))
    parser.add_argument("--ovmf-vars", type=Path, default=Path("artifacts/ovmf/OVMF_VARS.fd"))
    parser.add_argument("--out-dir", type=Path, default=Path("reports/secureboot/ovmf_matrix"))
    parser.add_argument("--timeout-sec", type=int, default=25)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    iso = args.iso if args.iso.is_absolute() else root / args.iso
    ovmf_code = args.ovmf_code if args.ovmf_code.is_absolute() else root / args.ovmf_code
    ovmf_vars = args.ovmf_vars if args.ovmf_vars.is_absolute() else root / args.ovmf_vars
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    qemu = find_qemu()
    failures: List[str] = []
    rows: List[CaseResult] = []

    if args.dry_run:
        summary = {"generated_utc": utc_now(), "ok": True, "dry_run": True, "rows": []}
        (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
        print("secureboot-ovmf-matrix: DRY-RUN")
        print(f"summary={out_dir / 'summary.json'}")
        return 0

    if not qemu:
        failures.append("qemu not found")
    if not iso.exists():
        failures.append(f"iso missing: {iso}")
    if not ovmf_code.exists():
        failures.append(f"ovmf code missing: {ovmf_code}")
    if not ovmf_vars.exists():
        failures.append(f"ovmf vars missing: {ovmf_vars}")
    if failures:
        summary = {"generated_utc": utc_now(), "ok": False, "failures": failures, "rows": []}
        (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
        print("secureboot-ovmf-matrix: FAIL")
        print(f"summary={out_dir / 'summary.json'}")
        return 1

    rows.append(run_case(qemu, iso, ovmf_code, ovmf_vars, False, out_dir, args.timeout_sec))
    rows.append(run_case(qemu, iso, ovmf_code, ovmf_vars, True, out_dir, args.timeout_sec))

    summary = {
        "generated_utc": utc_now(),
        "ok": all(r.ok for r in rows),
        "rows": [asdict(r) for r in rows],
        "failures": [f"{r.name}: rc={r.rc} timeout={r.timeout}" for r in rows if not r.ok],
    }
    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print(f"secureboot-ovmf-matrix: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())

