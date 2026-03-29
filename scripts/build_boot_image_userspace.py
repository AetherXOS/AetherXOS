from __future__ import annotations

import subprocess
from pathlib import Path


def _detect_host_target() -> str:
    output = subprocess.run(
        ["rustc", "-vV"],
        capture_output=True,
        text=True,
        check=True,
    )
    for line in output.stdout.splitlines():
        if line.startswith("host: "):
            return line.split(": ", 1)[1].strip()
    raise RuntimeError("failed to detect rustc host target")


def ensure_generated_userspace_binaries(initramfs_dir: Path) -> None:
    userspace_dir = initramfs_dir / "usr" / "lib" / "hypercore"
    userspace_dir.mkdir(parents=True, exist_ok=True)

    repo_root = Path(__file__).resolve().parent.parent
    manifest_path = repo_root / "host_tools" / "userspace_codegen" / "Cargo.toml"
    snapshot_path = userspace_dir / "userspace-codegen-snapshot.json"
    host_target = _detect_host_target()
    subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "--manifest-path",
            str(manifest_path),
            "--target",
            host_target,
            "--",
            "--repo-root",
            str(repo_root),
            "--out",
            str(snapshot_path),
            "--emit-dir",
            str(userspace_dir),
            "--run-smoke",
        ],
        check=True,
    )
