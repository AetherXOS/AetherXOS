#!/usr/bin/env python3
"""
Ensure Limine binaries are available.

Expected outputs:
  - limine-bios-cd.bin
  - limine-bios.sys
  - limine-uefi-cd.bin
  - BOOTX64.EFI
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import tarfile
import urllib.request
import zipfile
from pathlib import Path
from typing import Optional


REQUIRED = ("limine-bios-cd.bin", "limine-bios.sys", "limine-uefi-cd.bin", "BOOTX64.EFI")
DEFAULT_PREBUILT_ZIP = "https://codeload.github.com/zinix-org/limine-binaries/zip/refs/heads/main"


def run(cmd: list[str], cwd: Path) -> int:
    proc = subprocess.run(cmd, cwd=str(cwd), check=False)
    return int(proc.returncode)


def has_required(dir_path: Path) -> bool:
    return all((dir_path / name).exists() for name in REQUIRED)


def find_required(root: Path) -> Optional[Path]:
    for candidate in [root] + [p for p in root.rglob("*") if p.is_dir()]:
        if has_required(candidate):
            return candidate
    return None


def fetch_latest_release_version() -> str:
    url = "https://api.github.com/repos/limine-bootloader/limine/releases/latest"
    with urllib.request.urlopen(url, timeout=30) as r:
        payload = json.loads(r.read().decode("utf-8"))
    tag = str(payload.get("tag_name", "")).strip()
    if not tag:
        raise RuntimeError("failed to read latest limine release tag")
    return tag.removeprefix("v")


def download_release_tar(version: str, out_tar: Path) -> None:
    url = f"https://github.com/limine-bootloader/limine/releases/download/v{version}/limine-{version}.tar.gz"
    out_tar.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(url, timeout=60) as r:
        data = r.read()
    out_tar.write_bytes(data)


def extract_tar(tar_path: Path, out_dir: Path) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    with tarfile.open(tar_path, "r:gz") as t:
        try:
            t.extractall(path=out_dir, filter="fully_trusted")
        except TypeError:
            t.extractall(path=out_dir)
        roots = [m.name.split("/", 1)[0] for m in t.getmembers() if "/" in m.name]
    root_name = roots[0] if roots else f"limine-{tar_path.stem}"
    return out_dir / root_name


def download_file(url: str, out_path: Path) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(url, timeout=60) as r:
        out_path.write_bytes(r.read())


def extract_zip(zip_path: Path, out_dir: Path) -> Path:
    out_dir.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(zip_path, "r") as zf:
        zf.extractall(path=out_dir)
        names = [n for n in zf.namelist() if "/" in n]
    root_name = names[0].split("/", 1)[0] if names else zip_path.stem
    return out_dir / root_name


def try_prebuilt_fallback(cache_dir: Path, out_dir: Path, prebuilt_zip_url: str) -> bool:
    zip_path = cache_dir / "limine-prebuilt-main.zip"
    if not zip_path.exists():
        download_file(prebuilt_zip_url, zip_path)
    src_root = extract_zip(zip_path, cache_dir / "prebuilt-src")
    found = find_required(src_root)
    if found is None:
        return False
    copy_required(found, out_dir)
    return True


def find_bash() -> Optional[Path]:
    candidates = [
        Path(r"C:\msys64\usr\bin\bash.exe"),
        Path(r"C:\Program Files\Git\usr\bin\bash.exe"),
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


def to_wsl_path(path: Path) -> str:
    raw = str(path.resolve()).replace("\\", "/")
    if len(raw) >= 2 and raw[1] == ":":
        return f"/mnt/{raw[0].lower()}{raw[2:]}"
    return raw


def sh_quote(value: str) -> str:
    return "'" + value.replace("'", "'\"'\"'") + "'"


def try_build_limine_wsl(src_dir: Path) -> bool:
    if os.name != "nt":
        return False
    if shutil.which("wsl.exe") is None:
        return False
    src_wsl = to_wsl_path(src_dir)
    cmd = (
        f"set -euo pipefail; cd {sh_quote(src_wsl)}; "
        "./bootstrap && "
        "CC_FOR_TARGET=gcc LD_FOR_TARGET=ld OBJCOPY_FOR_TARGET=objcopy OBJDUMP_FOR_TARGET=objdump READELF_FOR_TARGET=readelf "
        "./configure --enable-bios --enable-bios-cd --enable-uefi-x86-64 --enable-uefi-cd && make -j$(nproc || echo 2)"
    )
    distros = ("Ubuntu-24.04", "Ubuntu")
    for distro in distros:
        rc = run(["wsl.exe", "-d", distro, "--", "bash", "-lc", cmd], src_dir)
        if rc == 0:
            return True
    return False


def try_build_limine(src_dir: Path) -> bool:
    tool_env = (
        "STRIP=true "
        "CC_FOR_TARGET=gcc "
        "LD_FOR_TARGET=ld "
        "OBJCOPY_FOR_TARGET=llvm-objcopy "
        "OBJDUMP_FOR_TARGET=llvm-objdump "
        "READELF_FOR_TARGET=llvm-readobj"
    )
    # POSIX path
    if os.name != "nt":
        if shutil.which("sh") and shutil.which("make"):
            cmd = [
                "sh",
                "-lc",
                "export PATH=\"$PATH:/mingw64/bin\"; "
                f"./bootstrap && {tool_env} ./configure --enable-bios --enable-bios-cd --enable-uefi-x86-64 --enable-uefi-cd && make -j$(getconf _NPROCESSORS_ONLN || echo 2)",
            ]
            return run(cmd, src_dir) == 0
        return False

    # Windows: try bash environments
    bash = find_bash()
    if bash is None:
        return False
    src_posix = str(src_dir).replace("\\", "/")
    if ":" in src_posix:
        drive = src_posix[0].lower()
        src_posix = f"/{drive}{src_posix[2:]}"
    shell_cmd = (
        "export PATH=\"$PATH:/mingw64/bin:/c/Program Files (x86)/GnuWin32/bin\"; "
        f"cd '{src_posix}' && "
        f"./bootstrap && {tool_env} ./configure --enable-bios --enable-bios-cd --enable-uefi-x86-64 --enable-uefi-cd && make -j2"
    )
    return run([str(bash), "-lc", shell_cmd], src_dir) == 0


def copy_required(from_dir: Path, to_dir: Path) -> None:
    to_dir.mkdir(parents=True, exist_ok=True)
    for name in REQUIRED:
        shutil.copy2(from_dir / name, to_dir / name)


def main() -> int:
    parser = argparse.ArgumentParser(description="Ensure Limine binaries exist")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--out-dir", type=Path, default=Path("artifacts/limine/bin"))
    parser.add_argument("--cache-dir", type=Path, default=Path("artifacts/limine/cache"))
    parser.add_argument("--version", default="latest")
    parser.add_argument("--allow-build", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument(
        "--prebuilt-zip-url",
        default=DEFAULT_PREBUILT_ZIP,
        help="Fallback URL for prebuilt Limine binaries zip",
    )
    parser.add_argument(
        "--allow-wsl-build",
        action="store_true",
        help="On Windows, fall back to WSL build if native build cannot produce required artifacts",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    cache_dir = args.cache_dir if args.cache_dir.is_absolute() else root / args.cache_dir

    if has_required(out_dir):
        print(f"limine binaries: READY ({out_dir})")
        return 0

    version = args.version
    if version == "latest":
        if args.offline:
            print("limine binaries: FAIL (offline mode does not allow resolving latest version)")
            print("hint: pass --version <exact> or disable --offline")
            return 1
        version = fetch_latest_release_version()

    tar_path = cache_dir / f"limine-{version}.tar.gz"
    if not tar_path.exists():
        if args.offline:
            print(f"limine binaries: FAIL (offline mode and cache miss: {tar_path})")
            return 1
        download_release_tar(version, tar_path)

    src_root = extract_tar(tar_path, cache_dir / f"src-{version}")
    found = find_required(src_root)
    if found is not None:
        copy_required(found, out_dir)
        print(f"limine binaries: READY ({out_dir})")
        return 0

    if (not args.offline) and try_prebuilt_fallback(cache_dir, out_dir, args.prebuilt_zip_url):
        print(f"limine binaries: READY ({out_dir}) [prebuilt fallback]")
        return 0

    if not args.allow_build:
        print("limine binaries: MISSING (release tar has no prebuilt binaries)")
        print("hint: rerun with --allow-build (requires shell/make/gcc toolchain)")
        return 1

    built = try_build_limine(src_root)
    if not built:
        if args.allow_wsl_build and os.name == "nt":
            print("limine build: native failed, trying WSL fallback...")
            built = try_build_limine_wsl(src_root)
        if not built:
            print("limine binaries: FAIL (build failed; ensure make+gcc toolchain is available)")
            if os.name == "nt":
                print("hint: run powershell -ExecutionPolicy Bypass -File .\\scripts\\setup_limine.ps1 -InstallWslDeps")
            return 1

    found = find_required(src_root)
    if found is None and args.allow_wsl_build and os.name == "nt":
        print("limine build: artifacts missing after native build, trying WSL fallback...")
        if try_build_limine_wsl(src_root):
            found = find_required(src_root)

    if found is None:
        print("limine binaries: FAIL (build finished but required binaries not found)")
        if os.name == "nt":
            print("hint: on Windows, run with --allow-wsl-build and ensure WSL has: make gcc autoconf automake nasm mtools xorriso")
            print("hint: run powershell -ExecutionPolicy Bypass -File .\\scripts\\setup_limine.ps1 -InstallWslDeps")
        return 1

    copy_required(found, out_dir)
    print(f"limine binaries: READY ({out_dir})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
