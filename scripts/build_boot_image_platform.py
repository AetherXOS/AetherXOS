from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

PANIC_MARKERS = ("PANIC report:", "[KERNEL DUMP] panic_count=", "kernel panic")


def run(cmd: list[str], cwd: Path) -> None:
    proc = subprocess.run(cmd, cwd=str(cwd), check=False)
    if proc.returncode != 0:
        raise RuntimeError(f"command failed ({proc.returncode}): {' '.join(cmd)}")


def write_grub_fallback_cfg(out_cfg: Path, limine_efi_name: str) -> None:
    content = (
        "set default=0\n"
        "set timeout=0\n"
        "\n"
        "menuentry 'HyperCore (Limine chain)' {\n"
        "  if [ -f ($root)/EFI/BOOT/{0} ]; then\n"
        "    chainloader /EFI/BOOT/{0}\n"
        "    boot\n"
        "  fi\n"
        "}\n"
    ).format(limine_efi_name)
    out_cfg.parent.mkdir(parents=True, exist_ok=True)
    out_cfg.write_text(content, encoding="utf-8")


def which(binary: str) -> bool:
    probe = ["where", binary] if os.name == "nt" else ["which", binary]
    proc = subprocess.run(probe, capture_output=True, text=True, check=False)
    if proc.returncode == 0 and bool(proc.stdout.strip()):
        return True
    if os.name == "nt" and binary == "qemu-system-x86_64":
        candidate = Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "qemu" / "qemu-system-x86_64.exe"
        return candidate.exists()
    if os.name == "nt" and binary == "xorriso":
        candidate = Path(r"C:\msys64\usr\bin\xorriso.exe")
        return candidate.exists()
    return False


def build_iso(
    *,
    out_iso: Path,
    stage_boot_dir: Path,
    limine_bin_dir: Path,
    shim_bin_dir: Optional[Path],
    shim_chainloader: str,
    grub_limine_target: str,
    write_grub_fallback: bool,
    root: Path,
) -> None:
    if not which("xorriso"):
        raise RuntimeError("xorriso not found in PATH")
    xorriso_bin = "xorriso"
    use_msys_xorriso = False
    if os.name == "nt":
        msys_xorriso = Path(r"C:\msys64\usr\bin\xorriso.exe")
        if msys_xorriso.exists():
            xorriso_bin = str(msys_xorriso)
            use_msys_xorriso = True

    def to_msys_path(path: Path) -> str:
        raw = str(path.resolve()).replace("\\", "/")
        if len(raw) >= 2 and raw[1] == ":":
            return f"/{raw[0].lower()}{raw[2:]}"
        return raw

    required = (
        "limine-bios-cd.bin",
        "limine-bios.sys",
        "limine-uefi-cd.bin",
        "BOOTX64.EFI",
    )
    for name in required:
        if not (limine_bin_dir / name).exists():
            raise FileNotFoundError(f"missing Limine binary: {limine_bin_dir / name}")

    iso_root = out_iso.parent / "iso_root"
    if iso_root.exists():
        shutil.rmtree(iso_root)
    (iso_root / "boot").mkdir(parents=True, exist_ok=True)
    (iso_root / "EFI" / "BOOT").mkdir(parents=True, exist_ok=True)

    for item in stage_boot_dir.iterdir():
        shutil.copy2(item, iso_root / "boot" / item.name)
        if item.name.lower() == "limine.conf":
            shutil.copy2(item, iso_root / "boot" / "limine.conf")
            shutil.copy2(item, iso_root / "limine.conf")

    shutil.copy2(limine_bin_dir / "limine-bios-cd.bin", iso_root / "boot" / "limine-bios-cd.bin")
    shutil.copy2(limine_bin_dir / "limine-bios.sys", iso_root / "boot" / "limine-bios.sys")
    shutil.copy2(limine_bin_dir / "limine-bios.sys", iso_root / "limine-bios.sys")
    shutil.copy2(limine_bin_dir / "limine-uefi-cd.bin", iso_root / "boot" / "limine-uefi-cd.bin")

    limine_efi = limine_bin_dir / "BOOTX64.EFI"
    if shim_bin_dir is not None:
        required_shim = ("shimx64.efi", "mmx64.efi")
        for name in required_shim:
            if not (shim_bin_dir / name).exists():
                raise FileNotFoundError(f"missing shim binary: {shim_bin_dir / name}")

        chain_name = (shim_chainloader or "grubx64.efi").strip()
        if "/" in chain_name or "\\" in chain_name:
            raise ValueError("--shim-chainloader must be a filename (no path separators)")
        limine_target_name = (grub_limine_target or "liminex64.efi").strip()
        if "/" in limine_target_name or "\\" in limine_target_name:
            raise ValueError("--grub-limine-target must be a filename (no path separators)")

        shutil.copy2(shim_bin_dir / "shimx64.efi", iso_root / "EFI" / "BOOT" / "BOOTX64.EFI")
        shutil.copy2(shim_bin_dir / "mmx64.efi", iso_root / "EFI" / "BOOT" / "mmx64.efi")
        fallback = shim_bin_dir / "fbx64.efi"
        if fallback.exists():
            shutil.copy2(fallback, iso_root / "EFI" / "BOOT" / "fbx64.efi")

        # If user provides chainloader EFI binary (e.g. grubx64.efi), use it.
        # Otherwise fall back to Limine EFI directly for compatibility.
        chainloader_src = shim_bin_dir / chain_name
        if chainloader_src.exists():
            shutil.copy2(chainloader_src, iso_root / "EFI" / "BOOT" / chain_name)
            shutil.copy2(limine_efi, iso_root / "EFI" / "BOOT" / limine_target_name)
            if write_grub_fallback and chain_name.lower().startswith("grub"):
                write_grub_fallback_cfg(iso_root / "EFI" / "BOOT" / "grub.cfg", limine_target_name)
        else:
            shutil.copy2(limine_efi, iso_root / "EFI" / "BOOT" / chain_name)
        notes = (
            "HyperCore shim+limine mode (experimental)\n"
            f"shim chain target: EFI/BOOT/{chain_name}\n"
            f"grub limine target: EFI/BOOT/{limine_target_name}\n"
            "Secure Boot note: this does NOT make boot chain trusted by itself.\n"
            "You still need properly signed EFI binaries and key enrollment.\n"
        )
        (iso_root / "EFI" / "BOOT" / "SHIM_NOTES.txt").write_text(notes, encoding="utf-8")
    else:
        shutil.copy2(limine_efi, iso_root / "EFI" / "BOOT" / "BOOTX64.EFI")

    out_iso_arg = str(out_iso)
    iso_root_arg = str(iso_root)
    if use_msys_xorriso:
        out_iso_arg = to_msys_path(out_iso)
        iso_root_arg = to_msys_path(iso_root)

    out_iso.parent.mkdir(parents=True, exist_ok=True)
    cmd = [
        xorriso_bin,
        "-as",
        "mkisofs",
        "-b",
        "boot/limine-bios-cd.bin",
        "-no-emul-boot",
        "-boot-load-size",
        "4",
        "-boot-info-table",
        "--efi-boot",
        "boot/limine-uefi-cd.bin",
        "-efi-boot-part",
        "--efi-boot-image",
        "--protective-msdos-label",
        "-o",
        out_iso_arg,
        iso_root_arg,
    ]
    run(cmd, root)


def ensure_limine_binaries(
    *,
    root: Path,
    out_dir: Path,
    cache_dir: Path,
    version: str,
    allow_build: bool,
    allow_wsl_build: bool,
) -> Path:
    cmd = [
        "python",
        "scripts/tools/ensure_limine_binaries.py",
        "--out-dir",
        str(out_dir),
        "--cache-dir",
        str(cache_dir),
        "--version",
        version,
    ]
    if allow_build:
        cmd.append("--allow-build")
    if allow_wsl_build:
        cmd.append("--allow-wsl-build")
    run(cmd, root)
    return out_dir


def run_qemu_smoke(
    *,
    kernel: Path,
    initramfs: Path,
    append: str,
    memory_mb: int,
    cores: int,
    timeout_sec: int,
    log_path: Path,
) -> None:
    qemu_bin = "qemu-system-x86_64"
    if not which(qemu_bin):
        raise RuntimeError("qemu-system-x86_64 not found in PATH")
    if os.name == "nt":
        qemu_candidate = Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "qemu" / "qemu-system-x86_64.exe"
        if qemu_candidate.exists():
            qemu_bin = str(qemu_candidate)

    cmd = [
        qemu_bin,
        "-nographic",
        "-m",
        str(memory_mb),
        "-smp",
        str(cores),
        "-kernel",
        str(kernel),
        "-initrd",
        str(initramfs),
        "-append",
        append,
    ]
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    timeout_seen = False
    output = ""
    try:
        output, _ = proc.communicate(timeout=timeout_sec)
    except subprocess.TimeoutExpired:
        timeout_seen = True
        proc.terminate()
        output, _ = proc.communicate(timeout=5)

    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(output, encoding="utf-8", errors="replace")
    panic_seen = any(marker in output for marker in PANIC_MARKERS)
    rc = proc.returncode if proc.returncode is not None else -1
    if timeout_seen or panic_seen or rc != 0:
        raise RuntimeError(
            f"qemu smoke failed rc={rc} timeout={timeout_seen} panic={panic_seen} log={log_path}"
        )

