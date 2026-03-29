#!/usr/bin/env python3
"""
Build minimal boot artifacts for HyperCore:
  - kernel ELF (cargo build)
  - initramfs.cpio.gz (newc format)
  - limine.cfg
  - optional ISO image (if host tools + Limine binaries are provided)
  - optional direct QEMU smoke boot (-kernel/-initrd)
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import List, Optional

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from limine_layout import append_probe_kernel_args, write_limine_config
from build_boot_image_initramfs import build_initramfs_newc
from build_boot_image_userspace import ensure_generated_userspace_binaries
from build_boot_image_platform import (
    build_iso,
    ensure_limine_binaries,
    run_qemu_smoke,
    which,
    write_grub_fallback_cfg,
)


DEFAULT_APPEND = "console=ttyS0 loglevel=7"


def run(cmd: List[str], cwd: Path) -> None:
    proc = subprocess.run(cmd, cwd=str(cwd), check=False)
    if proc.returncode != 0:
        raise RuntimeError(f"command failed ({proc.returncode}): {' '.join(cmd)}")


def copy2_resilient(src: Path, dst: Path, retries: int = 5, delay_sec: float = 0.2) -> None:
    last_error: Optional[Exception] = None
    for _ in range(retries):
        try:
            shutil.copy2(src, dst)
            return
        except OSError as exc:
            last_error = exc
            time.sleep(delay_sec)

    # Fallback to stream copy when Windows metadata-preserving copy hits a section lock.
    try:
        data = src.read_bytes()
        dst.write_bytes(data)
        return
    except OSError as exc:
        last_error = exc

    raise RuntimeError(f"failed to copy {src} -> {dst}: {last_error}")


def parse_feature_csv(raw: str) -> List[str]:
    features: List[str] = []
    for part in str(raw or "").split(","):
        item = part.strip()
        if item:
            features.append(item)
    # Keep order deterministic while preserving user sequence.
    seen = set()
    deduped: List[str] = []
    for item in features:
        if item in seen:
            continue
        seen.add(item)
        deduped.append(item)
    return deduped


def find_elf_artifact(root: Path, target: str, profile: str) -> Path:
    profile_dir = "release" if profile == "release" else "debug"
    search_dir = root / "target" / target / profile_dir
    if not search_dir.exists():
        raise FileNotFoundError(f"target dir not found: {search_dir}")
    candidates = [p for p in search_dir.iterdir() if p.is_file() and p.stat().st_size > 1024]
    for file in candidates:
        with file.open("rb") as f:
            if f.read(4) == b"\x7fELF":
                return file
    raise FileNotFoundError(f"no ELF artifact found under {search_dir}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build HyperCore boot artifacts")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--target", default="x86_64-unknown-none")
    parser.add_argument("--profile", choices=("debug", "release"), default="release")
    parser.add_argument(
        "--cargo-features",
        default="",
        help="Comma-separated Cargo features passed to cargo build.",
    )
    parser.add_argument(
        "--cargo-no-default-features",
        action="store_true",
        help="Build with --no-default-features.",
    )
    parser.add_argument(
        "--initramfs-dir",
        type=Path,
        default=Path("boot/initramfs"),
        help="Directory tree to pack as initramfs (cpio newc + gzip)",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("artifacts/boot_image"),
    )
    parser.add_argument("--append", default=DEFAULT_APPEND)
    parser.add_argument("--build-iso", action="store_true")
    parser.add_argument(
        "--limine-bin-dir",
        type=Path,
        default=None,
        help="Directory with limine-bios-cd.bin, limine-uefi-cd.bin, BOOTX64.EFI",
    )
    parser.add_argument(
        "--auto-fetch-limine",
        action="store_true",
        help="Auto download/prepare Limine binaries when --build-iso is used",
    )
    parser.add_argument("--limine-version", default="latest")
    parser.add_argument(
        "--limine-cache-dir",
        type=Path,
        default=Path("artifacts/limine/cache"),
    )
    parser.add_argument(
        "--limine-out-dir",
        type=Path,
        default=Path("artifacts/limine/bin"),
    )
    parser.add_argument(
        "--allow-build-limine",
        action="store_true",
        help="Allow building Limine from source when prebuilt binaries are absent",
    )
    parser.add_argument(
        "--allow-wsl-build-limine",
        action="store_true",
        help="On Windows, allow WSL fallback when native Limine build cannot produce required artifacts",
    )
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
        "--iso-name",
        default="hypercore.iso",
    )
    parser.add_argument("--qemu-smoke", action="store_true")
    parser.add_argument("--qemu-memory-mb", type=int, default=512)
    parser.add_argument("--qemu-cores", type=int, default=2)
    parser.add_argument("--qemu-timeout-sec", type=int, default=20)
    parser.add_argument(
        "--ab-slot",
        choices=("A", "B"),
        default=None,
        help="Optional A/B slot to stage generated boot artifacts",
    )
    parser.add_argument(
        "--ab-root",
        type=Path,
        default=Path("artifacts/boot_ab"),
        help="A/B slot metadata/artifact root",
    )
    parser.add_argument(
        "--ab-version",
        default=None,
        help="Optional version label recorded in A/B slot metadata",
    )
    parser.add_argument(
        "--ab-promote",
        action="store_true",
        help="Promote staged slot to active (pending validation)",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    initramfs_dir = args.initramfs_dir if args.initramfs_dir.is_absolute() else root / args.initramfs_dir

    build_cmd = ["cargo", "build", "--target", args.target]
    if args.profile == "release":
        build_cmd.append("--release")
    if args.cargo_no_default_features:
        build_cmd.append("--no-default-features")
    cargo_features = parse_feature_csv(args.cargo_features)
    if cargo_features:
        build_cmd.extend(["--features", ",".join(cargo_features)])
    run(build_cmd, root)

    kernel_elf = find_elf_artifact(root, args.target, args.profile)
    ensure_generated_userspace_binaries(initramfs_dir)

    stage_boot = out_dir / "stage" / "boot"
    stage_boot.mkdir(parents=True, exist_ok=True)
    stage_kernel = stage_boot / "hypercore.elf"
    stage_initramfs = stage_boot / "initramfs.cpio.gz"
    stage_limine_cfg = stage_boot / "limine.conf"
    stage_probe_limine_cfg = stage_boot / "limine-probe.conf"

    copy2_resilient(kernel_elf, stage_kernel)
    build_initramfs_newc(initramfs_dir, stage_initramfs)
    write_limine_config(
        stage_limine_cfg,
        kernel_name=stage_kernel.name,
        initramfs_name=stage_initramfs.name,
        append=args.append,
    )
    write_limine_config(
        stage_probe_limine_cfg,
        kernel_name=stage_kernel.name,
        initramfs_name=stage_initramfs.name,
        append=append_probe_kernel_args(args.append),
    )

    iso_path = None
    probe_iso_path = None
    if args.build_iso:
        if args.limine_bin_dir is not None:
            limine_bin_dir = (
                args.limine_bin_dir if args.limine_bin_dir.is_absolute() else root / args.limine_bin_dir
            )
        elif args.auto_fetch_limine:
            limine_out_dir = (
                args.limine_out_dir if args.limine_out_dir.is_absolute() else root / args.limine_out_dir
            )
            limine_cache_dir = (
                args.limine_cache_dir if args.limine_cache_dir.is_absolute() else root / args.limine_cache_dir
            )
            limine_bin_dir = ensure_limine_binaries(
                root=root,
                out_dir=limine_out_dir,
                cache_dir=limine_cache_dir,
                version=args.limine_version,
                allow_build=bool(args.allow_build_limine),
                allow_wsl_build=bool(args.allow_wsl_build_limine),
            )
        else:
            raise RuntimeError("--build-iso requires --limine-bin-dir or --auto-fetch-limine")
        iso_path = out_dir / args.iso_name
        build_iso(
            out_iso=iso_path,
            stage_boot_dir=stage_boot,
            limine_bin_dir=limine_bin_dir,
            shim_bin_dir=(
                args.shim_bin_dir if (args.shim_bin_dir and args.shim_bin_dir.is_absolute()) else
                (root / args.shim_bin_dir if args.shim_bin_dir else None)
            ),
            shim_chainloader=args.shim_chainloader,
            grub_limine_target=args.grub_limine_target,
            write_grub_fallback=bool(args.write_grub_fallback),
            root=root,
        )
        probe_iso_path = out_dir / f"{Path(args.iso_name).stem}-probe{Path(args.iso_name).suffix or '.iso'}"
        probe_stage_boot = out_dir / "probe_stage" / "boot"
        if probe_stage_boot.exists():
            shutil.rmtree(probe_stage_boot.parent)
        probe_stage_boot.mkdir(parents=True, exist_ok=True)
        copy2_resilient(stage_kernel, probe_stage_boot / stage_kernel.name)
        copy2_resilient(stage_initramfs, probe_stage_boot / stage_initramfs.name)
        copy2_resilient(stage_probe_limine_cfg, probe_stage_boot / "limine.conf")
        build_iso(
            out_iso=probe_iso_path,
            stage_boot_dir=probe_stage_boot,
            limine_bin_dir=limine_bin_dir,
            shim_bin_dir=(
                args.shim_bin_dir if (args.shim_bin_dir and args.shim_bin_dir.is_absolute()) else
                (root / args.shim_bin_dir if args.shim_bin_dir else None)
            ),
            shim_chainloader=args.shim_chainloader,
            grub_limine_target=args.grub_limine_target,
            write_grub_fallback=bool(args.write_grub_fallback),
            root=root,
        )

    if args.qemu_smoke:
        run_qemu_smoke(
            kernel=stage_kernel,
            initramfs=stage_initramfs,
            append=args.append,
            memory_mb=args.qemu_memory_mb,
            cores=args.qemu_cores,
            timeout_sec=args.qemu_timeout_sec,
            log_path=out_dir / "qemu_smoke.log",
        )

    ab_state_path = None
    if args.ab_slot is not None:
        ab_root = args.ab_root if args.ab_root.is_absolute() else root / args.ab_root
        version = args.ab_version or f"{args.profile}-{int(time.time())}"
        init_cmd = ["python", "scripts/ab_boot_slots.py", "--ab-root", str(ab_root), "init"]
        run(init_cmd, root)
        stage_cmd = [
            "python",
            "scripts/ab_boot_slots.py",
            "--ab-root",
            str(ab_root),
            "stage",
            "--slot",
            args.ab_slot,
            "--kernel",
            str(stage_kernel),
            "--initramfs",
            str(stage_initramfs),
            "--limine-cfg",
            str(stage_limine_cfg),
            "--version",
            version,
        ]
        if args.ab_promote:
            stage_cmd.append("--promote")
        run(stage_cmd, root)
        ab_state_path = ab_root / "state.json"

    print("boot image: PASS")
    print(f"kernel={stage_kernel}")
    print(f"initramfs={stage_initramfs}")
    print(f"limine_cfg={stage_limine_cfg}")
    print(f"limine_probe_cfg={stage_probe_limine_cfg}")
    if ab_state_path is not None:
        print(f"ab_state={ab_state_path}")
    if iso_path is not None:
        print(f"iso={iso_path}")
    if probe_iso_path is not None:
        print(f"probe_iso={probe_iso_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())




