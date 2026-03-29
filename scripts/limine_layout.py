from __future__ import annotations

from pathlib import Path


def render_limine_config(kernel_name: str, initramfs_name: str, append: str) -> str:
    return (
        "default_entry: 1\n"
        "timeout: 0\n"
        "verbose: yes\n"
        "serial: yes\n"
        "serial_baudrate: 115200\n"
        "graphics: no\n"
        "\n"
        "/HyperCore\n"
        "    protocol: limine\n"
        f"    kernel_path: boot():/boot/{kernel_name}\n"
        f"    module_path: boot():/boot/{initramfs_name}\n"
        f"    kernel_cmdline: {append}\n"
    )


def write_limine_config(out_cfg: Path, kernel_name: str, initramfs_name: str, append: str) -> None:
    out_cfg.parent.mkdir(parents=True, exist_ok=True)
    out_cfg.write_text(
        render_limine_config(kernel_name=kernel_name, initramfs_name=initramfs_name, append=append),
        encoding="utf-8",
    )


def append_probe_kernel_args(append: str) -> str:
    probe_flag = "HYPERCORE_RUN_LINKED_PROBE=1"
    tokens = append.split()
    if probe_flag in tokens:
        return append
    return f"{append} {probe_flag}".strip()

