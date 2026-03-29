#!/usr/bin/env python3
"""
Driver config smoke checker.

Validates that Cargo metadata driver defaults are present and match generated
driver constants consumed at runtime.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore


CONST_RE = re.compile(r"pub const ([A-Z0-9_]+): (?:u64|usize) = (\d+);")


def parse_numeric_consts(path: Path) -> dict[str, int]:
    text = path.read_text(encoding="utf-8", errors="replace")
    return {name: int(value) for name, value in CONST_RE.findall(text)}


def parse_driver_metadata(path: Path) -> dict[str, int]:
    data = tomllib.loads(path.read_text(encoding="utf-8", errors="replace"))
    return data["package"]["metadata"]["hypercore"]["config"]["drivers"]


def main() -> int:
    parser = argparse.ArgumentParser(description="Driver metadata/gen-const smoke checker")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument("--json", action="store_true", help="Emit JSON output")
    args = parser.parse_args()

    cargo = parse_driver_metadata(args.root / "Cargo.toml")
    generated = parse_numeric_consts(args.root / "src/generated_consts.rs")

    key_to_const = {
        "network_irq_service_budget": "DRIVER_NETWORK_IRQ_SERVICE_BUDGET",
        "network_loop_service_budget": "DRIVER_NETWORK_LOOP_SERVICE_BUDGET",
        "network_ring_limit": "DRIVER_NETWORK_RING_LIMIT",
        "network_quarantine_rebind_failures": "DRIVER_NETWORK_QUARANTINE_REBIND_FAILURES",
        "network_quarantine_cooldown_samples": "DRIVER_NETWORK_QUARANTINE_COOLDOWN_SAMPLES",
        "network_slo_max_drop_rate_per_mille": "DRIVER_NETWORK_SLO_MAX_DROP_RATE_PER_MILLE",
        "network_slo_max_tx_ring_utilization_percent": "DRIVER_NETWORK_SLO_MAX_TX_RING_UTILIZATION_PERCENT",
        "network_slo_max_rx_ring_utilization_percent": "DRIVER_NETWORK_SLO_MAX_RX_RING_UTILIZATION_PERCENT",
        "network_slo_max_io_errors": "DRIVER_NETWORK_SLO_MAX_IO_ERRORS",
        "network_low_latency_irq_budget_divisor": "DRIVER_NETWORK_LOW_LATENCY_IRQ_BUDGET_DIVISOR",
        "network_low_latency_loop_budget_divisor": "DRIVER_NETWORK_LOW_LATENCY_LOOP_BUDGET_DIVISOR",
        "network_low_latency_ring_limit_divisor": "DRIVER_NETWORK_LOW_LATENCY_RING_LIMIT_DIVISOR",
        "network_throughput_irq_budget_multiplier": "DRIVER_NETWORK_THROUGHPUT_IRQ_BUDGET_MULTIPLIER",
        "network_throughput_loop_budget_multiplier": "DRIVER_NETWORK_THROUGHPUT_LOOP_BUDGET_MULTIPLIER",
        "network_throughput_ring_limit_multiplier": "DRIVER_NETWORK_THROUGHPUT_RING_LIMIT_MULTIPLIER",
        "ahci_io_timeout_spins": "DRIVER_AHCI_IO_TIMEOUT_SPINS",
        "nvme_disable_ready_timeout_spins": "DRIVER_NVME_DISABLE_READY_TIMEOUT_SPINS",
        "nvme_poll_timeout_spins": "DRIVER_NVME_POLL_TIMEOUT_SPINS",
        "nvme_io_timeout_spins": "DRIVER_NVME_IO_TIMEOUT_SPINS",
        "e1000_reset_timeout_spins": "DRIVER_E1000_RESET_TIMEOUT_SPINS",
        "e1000_buffer_size_bytes": "DRIVER_E1000_BUFFER_SIZE_BYTES",
        "e1000_rx_desc_count": "DRIVER_E1000_RX_DESC_COUNT",
        "e1000_tx_desc_count": "DRIVER_E1000_TX_DESC_COUNT",
    }

    failures: list[str] = []
    checked: dict[str, int | None] = {}

    for key, const_name in key_to_const.items():
        metadata_value = cargo.get(key)
        if metadata_value is None:
            failures.append(f"missing metadata key: {key}")
            checked[const_name] = None
            continue
        generated_value = generated.get(const_name)
        checked[const_name] = generated_value
        if generated_value is None:
            failures.append(f"missing generated const: {const_name}")
            continue
        if int(metadata_value) != int(generated_value):
            failures.append(
                f"mismatch {key}/{const_name}: metadata={metadata_value}, generated={generated_value}"
            )

    out = {"ok": len(failures) == 0, "failures": failures, "checked": checked}

    if args.json:
        print(json.dumps(out, indent=2))
    else:
        if out["ok"]:
            print("driver-config smoke: PASS")
            print(f"  checked={len(key_to_const)}")
        else:
            print("driver-config smoke: FAIL")
            for failure in failures:
                print(f"  - {failure}")

    return 0 if out["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
