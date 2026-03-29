#!/usr/bin/env python3
"""
Decode GET_CORE_PRESSURE_SNAPSHOT syscall words into a readable report.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Dict, List, Optional


CORE_PRESSURE_SNAPSHOT_WORDS = 18
LOTTERY_REPLAY_LATEST_WORDS = 5

CORE_CLASS = {
    0: "Nominal",
    1: "Elevated",
    2: "High",
    3: "Critical",
}

SCHED_CLASS = {
    0: "Nominal",
    1: "Elevated",
    2: "High",
    3: "Critical",
}


def parse_int(value: str) -> int:
    return int(value.strip(), 0)


def parse_word_list(raw: Optional[str]) -> List[int]:
    if not raw:
        return []
    return [parse_int(part) for part in raw.split(",") if part.strip()]


def decode(words: List[int]) -> Dict[str, object]:
    if len(words) < CORE_PRESSURE_SNAPSHOT_WORDS:
        raise ValueError(
            f"GET_CORE_PRESSURE_SNAPSHOT requires at least {CORE_PRESSURE_SNAPSHOT_WORDS} words, got {len(words)}"
        )
    out = {
        "schema_version": words[0],
        "online_cpus": words[1],
        "runqueue_total": words[2],
        "runqueue_max": words[3],
        "runqueue_avg_milli": words[4],
        "rt_starvation_alert": words[5] != 0,
        "rt_forced_reschedules": words[6],
        "watchdog_stall_detections": words[7],
        "net_queue_limit": words[8],
        "net_rx_depth": words[9],
        "net_tx_depth": words[10],
        "net_saturation_percent": words[11],
        "lb_imbalance_p50": words[12],
        "lb_imbalance_p90": words[13],
        "lb_imbalance_p99": words[14],
        "lb_prefer_local_forced_moves": words[15],
        "core_pressure_class_raw": words[16],
        "scheduler_pressure_class_raw": words[17],
        "core_pressure_class": CORE_CLASS.get(words[16], "Unknown"),
        "scheduler_pressure_class": SCHED_CLASS.get(words[17], "Unknown"),
    }
    return out


def decode_lottery_replay(words: List[int]) -> Dict[str, object]:
    if len(words) < LOTTERY_REPLAY_LATEST_WORDS:
        raise ValueError(
            f"GET_LOTTERY_REPLAY_LATEST requires at least {LOTTERY_REPLAY_LATEST_WORDS} words, got {len(words)}"
        )
    return {
        "seq": words[0],
        "task_id": words[1],
        "winner_ticket": words[2],
        "total_tickets": words[3],
        "rng_state": f"0x{words[4]:x}",
    }


def render_markdown(data: Dict[str, object], replay: Optional[Dict[str, object]]) -> str:
    lines = ["# Core Pressure Snapshot Report", "", "## Core Pressure", ""]
    for k, v in data.items():
        lines.append(f"- {k}: `{v}`")
    lines.append("")
    if replay is not None:
        lines.append("## Lottery Replay")
        lines.append("")
        for k, v in replay.items():
            lines.append(f"- {k}: `{v}`")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Decode core pressure snapshot syscall words")
    parser.add_argument(
        "--words",
        required=True,
        help=(
            "Comma-separated usize words from GET_CORE_PRESSURE_SNAPSHOT (18 words). "
            "Example: 2,8,10,4,1250,0,12,0,1024,40,20,3,2,5,8,0,1,1"
        ),
    )
    parser.add_argument(
        "--lottery-replay-words",
        default=None,
        help=(
            "Optional comma-separated words from GET_LOTTERY_REPLAY_LATEST (5 words). "
            "Example: 17,42,120,2048,0x1234"
        ),
    )
    parser.add_argument("--format", choices=("md", "json"), default="md")
    parser.add_argument("--out", type=Path, default=None, help="Output file path")
    args = parser.parse_args()

    words = parse_word_list(args.words)
    data = decode(words)
    replay_words = parse_word_list(args.lottery_replay_words)
    replay_data = decode_lottery_replay(replay_words) if replay_words else None
    payload = {
        "core_pressure_snapshot": data,
        "lottery_replay_latest": replay_data,
    }
    rendered = render_markdown(data, replay_data) if args.format == "md" else json.dumps(payload, indent=2)

    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
