from __future__ import annotations

import gzip
import os
import stat
import time
from pathlib import Path
from typing import Iterable, Optional

from build_boot_image_userspace import ensure_generated_userspace_binaries
def align4(length: int) -> int:
    return (4 - (length % 4)) % 4


def newc_header(
    *,
    ino: int,
    mode: int,
    uid: int,
    gid: int,
    nlink: int,
    mtime: int,
    filesize: int,
    devmajor: int = 0,
    devminor: int = 0,
    rdevmajor: int = 0,
    rdevminor: int = 0,
    namesize: int = 0,
    check: int = 0,
) -> bytes:
    fields = [
        "070701",
        f"{ino:08x}",
        f"{mode:08x}",
        f"{uid:08x}",
        f"{gid:08x}",
        f"{nlink:08x}",
        f"{mtime:08x}",
        f"{filesize:08x}",
        f"{devmajor:08x}",
        f"{devminor:08x}",
        f"{rdevmajor:08x}",
        f"{rdevminor:08x}",
        f"{namesize:08x}",
        f"{check:08x}",
    ]
    return "".join(fields).encode("ascii")


def iter_cpio_entries(initramfs_dir: Path) -> Iterable[tuple[str, Path]]:
    # Root directory is implicit in initramfs; emit top-level children.
    for path in sorted(initramfs_dir.rglob("*")):
        rel = path.relative_to(initramfs_dir).as_posix()
        if rel:
            yield rel, path


def file_mode_for_entry(rel: str, path: Path, st_mode: int) -> int:
    if path.is_dir():
        return stat.S_IFDIR | (st_mode & 0o777)
    if path.is_symlink():
        return stat.S_IFLNK | 0o777
    # Windows checkout may not preserve executable bits. Force /init executable.
    if rel in (
        "init",
        "usr/bin/hyper_init",
        "usr/lib/hypercore/init",
        "usr/lib/hypercore/init.elf",
        "usr/lib/hypercore/probe.elf",
        "usr/lib/hypercore/probe-linked.elf",
        "usr/lib/hypercore/console.elf",
    ):
        return stat.S_IFREG | 0o755
    return stat.S_IFREG | (st_mode & 0o777)

def validate_initramfs_layout(initramfs_dir: Path) -> None:
    init_path = initramfs_dir / "init"
    if not init_path.exists():
        raise FileNotFoundError(f"initramfs is missing /init: {init_path}")

    required_dirs = [
        "bin",
        "dev",
        "etc",
        "proc",
        "run",
        "sys",
        "tmp",
        "usr",
        "usr/bin",
        "usr/lib",
        "usr/lib/hypercore",
        "var",
        "var/log",
    ]
    missing_dirs = [name for name in required_dirs if not (initramfs_dir / name).exists()]
    if missing_dirs:
        raise FileNotFoundError(
            "initramfs is missing required directories: " + ", ".join(sorted(missing_dirs))
        )

    try:
        head = init_path.read_bytes()[:256]
    except OSError as exc:
        raise RuntimeError(f"failed to read initramfs /init: {exc}") from exc

    if head.startswith(b"#!"):
        first_line = head.splitlines()[0].decode("utf-8", errors="ignore")[2:].strip()
        if first_line:
            interpreter = first_line.split()[0]
            if interpreter.startswith("/"):
                interpreter_path = initramfs_dir / interpreter.lstrip("/")
                if not interpreter_path.exists():
                    raise FileNotFoundError(
                        f"initramfs /init references missing interpreter {interpreter}"
                    )

    hyper_init = initramfs_dir / "usr" / "bin" / "hyper_init"
    userspace_init = initramfs_dir / "usr" / "lib" / "hypercore" / "init"
    userspace_init_elf = initramfs_dir / "usr" / "lib" / "hypercore" / "init.elf"
    shell_fallback = initramfs_dir / "bin" / "sh"
    if not hyper_init.exists() and not shell_fallback.exists():
        raise FileNotFoundError(
            "initramfs must provide /usr/bin/hyper_init or /bin/sh for early userspace"
        )
    if hyper_init.exists() and not userspace_init.exists() and not userspace_init_elf.exists() and not shell_fallback.exists():
        raise FileNotFoundError(
            "initramfs /usr/bin/hyper_init requires /usr/lib/hypercore/init(.elf) or /bin/sh"
        )

    profile = initramfs_dir / "etc" / "profile"
    if not profile.exists():
        raise FileNotFoundError(f"initramfs is missing /etc/profile: {profile}")


def build_initramfs_newc(initramfs_dir: Path, out_path: Path) -> None:
    if not initramfs_dir.exists():
        raise FileNotFoundError(f"initramfs dir not found: {initramfs_dir}")
    ensure_generated_userspace_binaries(initramfs_dir)
    validate_initramfs_layout(initramfs_dir)

    ino = 1
    now = int(time.time())
    buf = bytearray()

    def add_entry(name: str, path: Optional[Path], mode: int, data: bytes) -> None:
        nonlocal ino
        name_bytes = name.encode("utf-8") + b"\x00"
        hdr = newc_header(
            ino=ino,
            mode=mode,
            uid=0,
            gid=0,
            nlink=1,
            mtime=now,
            filesize=len(data),
            namesize=len(name_bytes),
        )
        ino += 1
        buf.extend(hdr)
        buf.extend(name_bytes)
        buf.extend(b"\x00" * align4(len(name_bytes)))
        if data:
            buf.extend(data)
            buf.extend(b"\x00" * align4(len(data)))

    for rel, path in iter_cpio_entries(initramfs_dir):
        st = path.lstat()
        if path.is_dir():
            mode = file_mode_for_entry(rel, path, st.st_mode)
            add_entry(rel, path, mode, b"")
        elif path.is_file():
            mode = file_mode_for_entry(rel, path, st.st_mode)
            add_entry(rel, path, mode, path.read_bytes())
        elif path.is_symlink():
            mode = file_mode_for_entry(rel, path, st.st_mode)
            target = os.readlink(path).encode("utf-8")
            add_entry(rel, path, mode, target)

    add_entry("TRAILER!!!", None, stat.S_IFREG, b"")

    out_path.parent.mkdir(parents=True, exist_ok=True)
    with out_path.open("wb") as raw:
        # Keep output deterministic across runs where supported.
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, compresslevel=9, mtime=0) as gz:
            gz.write(buf)

