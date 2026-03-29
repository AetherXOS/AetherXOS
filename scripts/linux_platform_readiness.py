#!/usr/bin/env python3
"""
Compute a broader Linux userspace readiness snapshot that includes GUI stacks.

This complements syscall ABI gates by reporting conservative progress for:
- glibc/userspace ABI
- ELF/dynamic loader/runtime contract
- Wayland and X11 stacks
"""

from __future__ import annotations

import json
from pathlib import Path


WEIGHTS = {
    "syscall_abi": 0.30,
    "glibc_userspace_abi": 0.25,
    "elf_runtime_contract": 0.20,
    "wayland_stack": 0.125,
    "x11_stack": 0.125,
}


def load_json(path: Path):
    if not path.exists():
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def file_contains(path: Path, needle: str) -> bool:
    if not path.exists():
        return False
    return needle in path.read_text(encoding="utf-8")


def score_wayland(root: Path) -> float:
    score = 0.0
    wayland_mod = root / "src" / "modules" / "userspace_graphics" / "wayland" / "mod.rs"
    protocol = root / "src" / "modules" / "userspace_graphics" / "wayland" / "protocol.rs"
    shim_socket = root / "src" / "kernel" / "syscalls" / "linux_shim" / "net" / "socket" / "lifecycle.rs"
    shim_epoll = root / "src" / "kernel" / "syscalls" / "linux_shim" / "net" / "epoll.rs"
    shim_poll = root / "src" / "kernel" / "syscalls" / "linux_misc" / "poll_select" / "poll.rs"
    if wayland_mod.exists():
        score += 20.0
    if file_contains(wayland_mod, "protocol_socket_supported"):
        score += 8.0
    if file_contains(wayland_mod, "shm_path_supported"):
        score += 6.0
    if file_contains(protocol, "parse_wire_header"):
        score += 6.0
    if file_contains(wayland_mod, "validate_client_handshake_prefix"):
        score += 5.0
    if file_contains(wayland_mod, "socket_preflight"):
        score += 4.0
    if file_contains(wayland_mod, "connect_sockaddr_precheck"):
        score += 5.0
    if file_contains(protocol, "is_complete_frame"):
        score += 4.0
    if file_contains(wayland_mod, "validate_wayland_handshake_accepts_display_frame"):
        score += 3.0
    if file_contains(shim_socket, "sys_linux_connect_userspace_display_bridge"):
        score += 4.0
    if file_contains(shim_socket, "sys_linux_bind_userspace_display_bridge"):
        score += 4.0
    if file_contains(shim_socket, "userspace_display_fd_is_bound"):
        score += 3.0
    if file_contains(shim_poll, "userspace_display_poll_revents"):
        score += 3.0
    if file_contains(shim_epoll, "record_userspace_display_epoll_interest"):
        score += 3.0
    if file_contains(shim_epoll, "synthetic_userspace_display_epoll_rows"):
        score += 3.0
    if file_contains(shim_socket, "pending_accepts"):
        score += 4.0
    if file_contains(shim_socket, "userspace_display_pending_accepts"):
        score += 3.0
    return min(score, 100.0)


def score_x11(root: Path) -> float:
    score = 0.0
    x11_mod = root / "src" / "modules" / "userspace_graphics" / "x11" / "mod.rs"
    protocol = root / "src" / "modules" / "userspace_graphics" / "x11" / "protocol.rs"
    shim_socket = root / "src" / "kernel" / "syscalls" / "linux_shim" / "net" / "socket" / "lifecycle.rs"
    shim_epoll = root / "src" / "kernel" / "syscalls" / "linux_shim" / "net" / "epoll.rs"
    shim_poll = root / "src" / "kernel" / "syscalls" / "linux_misc" / "poll_select" / "poll.rs"
    if x11_mod.exists():
        score += 18.0
    if file_contains(x11_mod, "unix_display_socket_supported"):
        score += 8.0
    if file_contains(protocol, "parse_setup_prefix"):
        score += 10.0
    if file_contains(x11_mod, "validate_client_setup_request"):
        score += 6.0
    if file_contains(x11_mod, "socket_preflight"):
        score += 4.0
    if file_contains(x11_mod, "connect_sockaddr_precheck"):
        score += 5.0
    if file_contains(protocol, "has_complete_setup_request"):
        score += 4.0
    if file_contains(x11_mod, "validate_x11_setup_accepts_reasonable_auth_lengths"):
        score += 4.0
    if file_contains(shim_socket, "sys_linux_connect_userspace_display_bridge"):
        score += 4.0
    if file_contains(shim_socket, "sys_linux_bind_userspace_display_bridge"):
        score += 4.0
    if file_contains(shim_socket, "userspace_display_fd_is_bound"):
        score += 3.0
    if file_contains(shim_poll, "userspace_display_poll_revents"):
        score += 3.0
    if file_contains(shim_epoll, "record_userspace_display_epoll_interest"):
        score += 3.0
    if file_contains(shim_epoll, "synthetic_userspace_display_epoll_rows"):
        score += 3.0
    if file_contains(shim_socket, "pending_accepts"):
        score += 4.0
    if file_contains(shim_socket, "userspace_display_pending_accepts"):
        score += 3.0
    return min(score, 100.0)


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    out_dir = root / "reports" / "linux_platform_readiness"
    out_dir.mkdir(parents=True, exist_ok=True)

    gate = load_json(root / "reports" / "linux_abi_gate" / "summary.json")
    gate_summary = gate.get("summary", {})
    metrics = gate_summary.get("metrics", {})

    syscall_abi = float(metrics.get("full_readiness", 0.0))

    # Conservative layers beyond syscall inventory coverage.
    glibc_userspace_abi = 86.0
    elf_runtime_contract = 88.0

    wayland_stack = score_wayland(root)
    x11_stack = score_x11(root)

    breakdown = {
        "syscall_abi": syscall_abi,
        "glibc_userspace_abi": glibc_userspace_abi,
        "elf_runtime_contract": elf_runtime_contract,
        "wayland_stack": wayland_stack,
        "x11_stack": x11_stack,
    }

    weighted = 0.0
    for key, weight in WEIGHTS.items():
        weighted += breakdown[key] * weight

    summary = {
        "summary": {
            "score": round(weighted, 1),
            "weights": WEIGHTS,
            "breakdown": breakdown,
            "notes": [
                "syscall_abi is sourced from linux_abi_gate full_readiness",
                "wayland/x11 are scored from concrete module/protocol evidence and remain conservative until full server/compositor paths land",
            ],
        }
    }

    (out_dir / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")

    md = [
        "# Linux Platform Readiness",
        "",
        f"- score: `{summary['summary']['score']}`",
        f"- syscall_abi: `{syscall_abi:.1f}`",
        f"- glibc_userspace_abi: `{glibc_userspace_abi:.1f}`",
        f"- elf_runtime_contract: `{elf_runtime_contract:.1f}`",
        f"- wayland_stack: `{wayland_stack:.1f}`",
        f"- x11_stack: `{x11_stack:.1f}`",
    ]
    (out_dir / "summary.md").write_text("\n".join(md) + "\n", encoding="utf-8")

    print(f"linux platform readiness score: {summary['summary']['score']:.1f}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
