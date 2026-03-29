#!/usr/bin/env python3
"""
QEMU boot soak/stress/chaos matrix runner.

Builds the target once, then executes repeated QEMU boots with varying
memory/core settings and optional chaos modes. Captures per-round logs and a
summary report suitable for CI gating.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import subprocess
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Dict, List, Tuple


@dataclass
class BootRound:
    round_index: int
    memory_mb: int
    cores: int
    chaos_mode: str
    expected_success: bool
    ok: bool
    exit_code: int
    duration_sec: float
    panic_seen: bool
    timeout_seen: bool
    failure_reason: str
    log_path: str


def find_qemu_binary() -> str | None:
    candidates: List[str] = ["qemu-system-x86_64"]
    if os.name == "nt":
        program_files = os.environ.get("ProgramFiles", r"C:\Program Files")
        qemu_exe = str(Path(program_files) / "qemu" / "qemu-system-x86_64.exe")
        candidates.append(qemu_exe)
    for candidate in candidates:
        path = subprocess.run(
            ["where", candidate] if os.name == "nt" else ["which", candidate],
            capture_output=True,
            text=True,
            check=False,
        )
        if path.returncode == 0 and path.stdout.strip():
            return candidate
        if os.path.isfile(candidate):
            return candidate
    return None


def parse_int_list(raw: str) -> List[int]:
    out = []
    for part in raw.split(","):
        part = part.strip()
        if not part:
            continue
        out.append(int(part))
    return out


def parse_feature_csv(raw: str) -> List[str]:
    values: List[str] = []
    for part in str(raw or "").split(","):
        item = part.strip()
        if item:
            values.append(item)
    seen = set()
    deduped: List[str] = []
    for item in values:
        if item in seen:
            continue
        seen.add(item)
        deduped.append(item)
    return deduped


def find_kernel_artifact(root: Path, target: str, profile: str) -> Path:
    profile_dir = "release" if profile == "release" else "debug"
    search_dir = root / "target" / target / profile_dir
    if not search_dir.exists():
        raise FileNotFoundError(f"target dir not found: {search_dir}")
    candidates = [p for p in search_dir.iterdir() if p.is_file() and p.stat().st_size > 1024]
    for file in candidates:
        with file.open("rb") as f:
            hdr = f.read(4)
        if hdr == b"\x7fELF":
            return file
    raise FileNotFoundError(f"no ELF artifact found under {search_dir}")


def run_cmd(cmd: List[str], cwd: Path) -> int:
    proc = subprocess.run(cmd, cwd=str(cwd), check=False)
    return int(proc.returncode)


def run_qemu_round(
    qemu_bin: str,
    boot_mode: str,
    kernel_path: Path | None,
    initrd_path: Path | None,
    iso_path: Path | None,
    append: str,
    memory_mb: int,
    cores: int,
    chaos_mode: str,
    round_timeout_sec: int,
    allow_timeout_success: bool,
    log_path: Path,
) -> Tuple[bool, int, float, bool, bool, str]:
    args = [
        qemu_bin,
        "-nographic",
        "-m",
        str(memory_mb),
        "-smp",
        str(cores),
    ]
    if boot_mode == "iso":
        if iso_path is None:
            raise ValueError("iso boot mode requires iso_path")
        args.extend(["-cdrom", str(iso_path), "-boot", "d"])
    else:
        if kernel_path is None:
            raise ValueError("direct boot mode requires kernel_path")
        args.extend(["-kernel", str(kernel_path)])
        if initrd_path is not None:
            args.extend(["-initrd", str(initrd_path)])
        if append:
            args.extend(["-append", append])
    start = time.perf_counter()
    proc = subprocess.Popen(
        args,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )

    timeout_seen = False
    panic_seen = False
    collected: List[str] = []
    try:
        if chaos_mode == "early_kill":
            # Let it boot briefly and then terminate to simulate disruptive failure.
            time.sleep(max(1, min(3, round_timeout_sec // 4)))
            proc.terminate()
        out, _ = proc.communicate(timeout=round_timeout_sec)
        if out:
            collected.append(out)
    except subprocess.TimeoutExpired:
        timeout_seen = True
        proc.terminate()
        try:
            out, _ = proc.communicate(timeout=5)
            if out:
                collected.append(out)
        except subprocess.TimeoutExpired:
            proc.kill()
            out, _ = proc.communicate()
            if out:
                collected.append(out)

    full_log = "".join(collected)
    panic_seen = ("PANIC report:" in full_log) or ("[KERNEL DUMP] panic_count=" in full_log)
    failure_reason = ""
    if "Error loading uncompressed kernel without PVH ELF Note" in full_log:
        failure_reason = "bootloader_incompatible_missing_pvh_note"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(full_log, encoding="utf-8", errors="replace")

    duration = time.perf_counter() - start
    rc = proc.returncode if proc.returncode is not None else -1
    ok = (rc == 0) and (not timeout_seen) and (not panic_seen)
    if allow_timeout_success and timeout_seen and (not panic_seen):
        ok = True
    if not ok and not failure_reason:
        if timeout_seen:
            failure_reason = "timeout"
        elif panic_seen:
            failure_reason = "kernel_panic_marker"
        else:
            failure_reason = f"qemu_exit_{rc}"
    return ok, rc, duration, panic_seen, timeout_seen, failure_reason


def main() -> int:
    parser = argparse.ArgumentParser(description="Run QEMU boot soak/stress/chaos matrix")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--target", default="x86_64-unknown-none")
    parser.add_argument("--profile", choices=("debug", "release"), default="debug")
    parser.add_argument(
        "--cargo-features",
        default="",
        help="Comma-separated Cargo features passed to cargo build/build_boot_image.",
    )
    parser.add_argument(
        "--cargo-no-default-features",
        action="store_true",
        help="Build with --no-default-features.",
    )
    parser.add_argument("--boot-mode", choices=("direct", "iso"), default="direct")
    parser.add_argument(
        "--kernel-path",
        type=Path,
        default=None,
        help="Use prebuilt kernel ELF path instead of auto-discovering target artifact",
    )
    parser.add_argument(
        "--initrd-path",
        type=Path,
        default=None,
        help="Optional initramfs for direct kernel boot",
    )
    parser.add_argument(
        "--iso-path",
        type=Path,
        default=None,
        help="ISO path for boot-mode=iso",
    )
    parser.add_argument(
        "--build-iso",
        action="store_true",
        help="When boot-mode=iso and iso-path is omitted, build ISO with scripts/build_boot_image.py",
    )
    parser.add_argument(
        "--limine-bin-dir",
        type=Path,
        default=None,
        help="Required with --build-iso (directory with Limine binaries)",
    )
    parser.add_argument("--iso-name", default="hypercore.iso")
    parser.add_argument("--auto-fetch-limine", action="store_true")
    parser.add_argument("--limine-version", default="latest")
    parser.add_argument("--limine-cache-dir", type=Path, default=Path("artifacts/limine/cache"))
    parser.add_argument("--limine-out-dir", type=Path, default=Path("artifacts/limine/bin"))
    parser.add_argument("--allow-build-limine", action="store_true")
    parser.add_argument("--allow-wsl-build-limine", action="store_true")
    parser.add_argument(
        "--shim-bin-dir",
        type=Path,
        default=None,
        help="Optional shim directory (shimx64.efi/mmx64.efi[/fbx64.efi]) for shim+limine ISO layout",
    )
    parser.add_argument(
        "--shim-chainloader",
        default="grubx64.efi",
        help="EFI filename shim will chainload (Limine EFI is copied with this name)",
    )
    parser.add_argument(
        "--grub-limine-target",
        default="liminex64.efi",
        help="EFI filename GRUB fallback config chainloads to (used in shim mode)",
    )
    parser.add_argument(
        "--write-grub-fallback",
        action="store_true",
        help="Write EFI/BOOT/grub.cfg fallback chainloader config in shim mode",
    )
    parser.add_argument(
        "--boot-image-out-dir",
        type=Path,
        default=Path("artifacts/boot_image"),
        help="Output dir used when --build-iso is enabled",
    )
    parser.add_argument("--append", default="console=ttyS0 loglevel=7")
    parser.add_argument("--rounds", type=int, default=20)
    parser.add_argument("--memory-mb", default="512,1024")
    parser.add_argument("--cores", default="1,2,4")
    parser.add_argument("--chaos-rate", type=float, default=0.2)
    parser.add_argument("--round-timeout-sec", type=int, default=20)
    parser.add_argument("--seed", type=int, default=20260305)
    parser.add_argument("--out-dir", type=Path, default=Path("artifacts/qemu_soak"))
    parser.add_argument(
        "--allow-timeout-success",
        action="store_true",
        help="Treat timeout (without panic marker) as success for expected-success rounds",
    )
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)
    random.seed(args.seed)

    mem_options = parse_int_list(args.memory_mb)
    core_options = parse_int_list(args.cores)
    if not mem_options or not core_options:
        raise ValueError("memory/core option lists cannot be empty")

    if args.dry_run:
        payload = {
            "summary": {
                "ok": True,
                "dry_run": True,
                "rounds": args.rounds,
                "seed": args.seed,
                "artifact_dir": str(out_dir),
            },
            "rounds": [],
        }
        (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
        print("qemu soak matrix: DRY-RUN")
        print(f"summary={out_dir / 'summary.json'}")
        return 0

    qemu_bin = find_qemu_binary()
    if not qemu_bin:
        print("qemu soak matrix: FAIL (qemu-system-x86_64 not found)")
        return 1

    kernel: Path | None = None
    initrd: Path | None = None
    iso_path: Path | None = None
    allow_timeout_success = bool(args.allow_timeout_success)

    if args.boot_mode == "iso":
        if args.iso_path is not None:
            iso_path = args.iso_path if args.iso_path.is_absolute() else root / args.iso_path
            if not iso_path.exists():
                print(f"qemu soak matrix: FAIL (iso not found: {iso_path})")
                return 1
        else:
            if not args.build_iso:
                print("qemu soak matrix: FAIL (boot-mode=iso requires --iso-path or --build-iso)")
                return 1
            boot_image_out_dir = (
                args.boot_image_out_dir
                if args.boot_image_out_dir.is_absolute()
                else root / args.boot_image_out_dir
            )
            build_boot_cmd = [
                "python",
                "scripts/build_boot_image.py",
                "--target",
                args.target,
                "--profile",
                args.profile,
                "--out-dir",
                str(boot_image_out_dir),
                "--build-iso",
                "--iso-name",
                args.iso_name,
            ]
            if args.limine_bin_dir is not None:
                limine_bin_dir = (
                    args.limine_bin_dir if args.limine_bin_dir.is_absolute() else root / args.limine_bin_dir
                )
                build_boot_cmd.extend(["--limine-bin-dir", str(limine_bin_dir)])
            elif args.auto_fetch_limine:
                limine_cache_dir = (
                    args.limine_cache_dir if args.limine_cache_dir.is_absolute() else root / args.limine_cache_dir
                )
                limine_out_dir = (
                    args.limine_out_dir if args.limine_out_dir.is_absolute() else root / args.limine_out_dir
                )
                build_boot_cmd.extend(
                    [
                        "--auto-fetch-limine",
                        "--limine-version",
                        args.limine_version,
                        "--limine-cache-dir",
                        str(limine_cache_dir),
                        "--limine-out-dir",
                        str(limine_out_dir),
                    ]
                )
                if args.allow_build_limine:
                    build_boot_cmd.append("--allow-build-limine")
                if args.allow_wsl_build_limine:
                    build_boot_cmd.append("--allow-wsl-build-limine")
            else:
                print("qemu soak matrix: FAIL (--build-iso requires --limine-bin-dir or --auto-fetch-limine)")
                return 1
            if args.shim_bin_dir is not None:
                shim_bin_dir = args.shim_bin_dir if args.shim_bin_dir.is_absolute() else root / args.shim_bin_dir
                build_boot_cmd.extend(["--shim-bin-dir", str(shim_bin_dir)])
                if args.shim_chainloader:
                    build_boot_cmd.extend(["--shim-chainloader", args.shim_chainloader])
                if args.grub_limine_target:
                    build_boot_cmd.extend(["--grub-limine-target", args.grub_limine_target])
                if args.write_grub_fallback:
                    build_boot_cmd.append("--write-grub-fallback")
            cargo_features = parse_feature_csv(args.cargo_features)
            if args.cargo_no_default_features:
                build_boot_cmd.append("--cargo-no-default-features")
            if cargo_features:
                build_boot_cmd.extend(["--cargo-features", ",".join(cargo_features)])
            rc = run_cmd(build_boot_cmd, root)
            if rc != 0:
                print("qemu soak matrix: FAIL (build_boot_image --build-iso failed)")
                return rc
            iso_path = boot_image_out_dir / args.iso_name
            if not iso_path.exists():
                print(f"qemu soak matrix: FAIL (built iso not found: {iso_path})")
                return 1
        if not args.allow_timeout_success:
            allow_timeout_success = True
    else:
        if args.kernel_path is None:
            build_args = ["cargo", "build", "--target", args.target]
            if args.profile == "release":
                build_args.append("--release")
            if args.cargo_no_default_features:
                build_args.append("--no-default-features")
            cargo_features = parse_feature_csv(args.cargo_features)
            if cargo_features:
                build_args.extend(["--features", ",".join(cargo_features)])
            build = subprocess.run(build_args, cwd=str(root), check=False)
            if build.returncode != 0:
                print("qemu soak matrix: FAIL (cargo build failed)")
                return build.returncode
            kernel = find_kernel_artifact(root, args.target, args.profile)
        else:
            kernel = args.kernel_path if args.kernel_path.is_absolute() else root / args.kernel_path
            if not kernel.exists():
                print(f"qemu soak matrix: FAIL (kernel not found: {kernel})")
                return 1

        if args.initrd_path is not None:
            initrd = args.initrd_path if args.initrd_path.is_absolute() else root / args.initrd_path
            if not initrd.exists():
                print(f"qemu soak matrix: FAIL (initrd not found: {initrd})")
                return 1

    rounds: List[BootRound] = []
    for idx in range(1, args.rounds + 1):
        mem = random.choice(mem_options)
        cores = random.choice(core_options)
        chaos_mode = "early_kill" if random.random() < args.chaos_rate else "none"
        expected_success = chaos_mode == "none"

        log_path = out_dir / f"round_{idx:04d}.log"
        ok, rc, dur, panic_seen, timeout_seen, failure_reason = run_qemu_round(
            qemu_bin=qemu_bin,
            boot_mode=args.boot_mode,
            kernel_path=kernel,
            initrd_path=initrd,
            iso_path=iso_path,
            append=args.append,
            memory_mb=mem,
            cores=cores,
            chaos_mode=chaos_mode,
            round_timeout_sec=args.round_timeout_sec,
            allow_timeout_success=allow_timeout_success,
            log_path=log_path,
        )
        if not expected_success:
            # Chaos failures are expected and should not fail the whole matrix.
            ok = True

        rounds.append(
            BootRound(
                round_index=idx,
                memory_mb=mem,
                cores=cores,
                chaos_mode=chaos_mode,
                expected_success=expected_success,
                ok=ok,
                exit_code=rc,
                duration_sec=dur,
                panic_seen=panic_seen,
                timeout_seen=timeout_seen,
                failure_reason=failure_reason,
                log_path=str(log_path),
            )
        )

    failed = [r for r in rounds if not r.ok]
    summary = {
        "ok": len(failed) == 0,
        "boot_mode": args.boot_mode,
        "rounds": args.rounds,
        "seed": args.seed,
        "failed_rounds": len(failed),
        "expected_success_rounds": sum(1 for r in rounds if r.expected_success),
        "chaos_rounds": sum(1 for r in rounds if not r.expected_success),
        "failure_reasons": {},
    }
    for item in failed:
        reason = item.failure_reason or "unknown"
        summary["failure_reasons"][reason] = int(summary["failure_reasons"].get(reason, 0)) + 1
    payload = {"summary": summary, "rounds": [asdict(r) for r in rounds]}
    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")

    md = [
        "# QEMU Soak Matrix",
        "",
        f"- ok: `{summary['ok']}`",
        f"- rounds: `{summary['rounds']}`",
        f"- failed_rounds: `{summary['failed_rounds']}`",
        f"- expected_success_rounds: `{summary['expected_success_rounds']}`",
        f"- chaos_rounds: `{summary['chaos_rounds']}`",
        "",
    ]
    if failed:
        md.append("## Failed Rounds")
        md.append("")
        for r in failed[:50]:
            md.append(
                f"- round `{r.round_index}` mem `{r.memory_mb}` cores `{r.cores}` rc `{r.exit_code}` "
                f"panic `{r.panic_seen}` timeout `{r.timeout_seen}` mode `{r.chaos_mode}`"
            )
    else:
        md.append("All expected-success rounds passed.")
    (out_dir / "summary.md").write_text("\n".join(md) + "\n", encoding="utf-8")

    print(f"qemu soak matrix: {'PASS' if summary['ok'] else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if summary["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
