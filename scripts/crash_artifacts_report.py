#!/usr/bin/env python3
"""
Kernel crash artifact reporter.

Inputs:
- Kernel text log file (serial/kmsg style).
- Optional raw syscall word dumps for GET_CRASH_REPORT / LIST_CRASH_EVENTS.
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Dict, List, Optional


CRASH_REPORT_WORDS = 10
CRASH_EVENT_WORDS = 8

EVENT_KIND_LABELS: Dict[int, str] = {
    0: "unknown",
    1: "panic",
    2: "soft_watchdog_stall",
    3: "hard_watchdog_stall",
    4: "driver_quarantine",
}

PANIC_RE = re.compile(
    r"PANIC report: count=(?P<count>\d+)\s+reason=(?P<reason>[^\s]+)\s+hash=(?P<hash>0x[0-9a-fA-F]+)"
)
KERNEL_PANIC_RE = re.compile(
    r"\[KERNEL DUMP\]\s+panic_count=(?P<count>\d+)\s+last_panic_tick=(?P<tick>\d+)\s+last_reason_hash=(?P<hash>0x[0-9a-fA-F]+)"
)
KERNEL_EVENT_RE = re.compile(
    r"\[KERNEL DUMP\]\s+crash_event\s+seq=(?P<seq>\d+)\s+kind=(?P<kind>\d+)\s+tick=(?P<tick>\d+)\s+cpu=(?P<cpu>\d+)\s+task=(?P<task>\d+)\s+reason_hash=(?P<hash>0x[0-9a-fA-F]+)\s+aux0=(?P<aux0>\d+)\s+aux1=(?P<aux1>\d+)"
)


@dataclass
class CrashEvent:
    seq: int
    kind: int
    kind_label: str
    tick: int
    cpu_id: int
    task_id: int
    reason_hash: str
    aux0: int
    aux1: int


@dataclass
class CrashReport:
    panic_count: int
    last_panic_tick: int
    last_reason_hash: str
    watchdog_tick: int
    watchdog_stalls: int
    watchdog_hard_panics: int
    startup_stage_transitions: int
    startup_order_violations: int
    crash_log_latest_seq: int
    crash_log_latest_kind: int
    crash_log_latest_kind_label: str


def parse_int(value: str) -> int:
    return int(value.strip(), 0)


def parse_word_list(raw: Optional[str]) -> List[int]:
    if not raw:
        return []
    return [parse_int(part) for part in raw.split(",") if part.strip()]


def decode_crash_report_words(words: List[int]) -> Optional[CrashReport]:
    if not words:
        return None
    if len(words) < CRASH_REPORT_WORDS:
        raise ValueError(
            f"GET_CRASH_REPORT requires at least {CRASH_REPORT_WORDS} words, got {len(words)}"
        )
    latest_kind = words[9]
    return CrashReport(
        panic_count=words[0],
        last_panic_tick=words[1],
        last_reason_hash=f"0x{words[2]:x}",
        watchdog_tick=words[3],
        watchdog_stalls=words[4],
        watchdog_hard_panics=words[5],
        startup_stage_transitions=words[6],
        startup_order_violations=words[7],
        crash_log_latest_seq=words[8],
        crash_log_latest_kind=latest_kind,
        crash_log_latest_kind_label=EVENT_KIND_LABELS.get(latest_kind, "unknown"),
    )


def decode_crash_event_words(words: List[int]) -> List[CrashEvent]:
    if not words:
        return []
    if len(words) % CRASH_EVENT_WORDS != 0:
        raise ValueError(
            f"LIST_CRASH_EVENTS words must be a multiple of {CRASH_EVENT_WORDS}, got {len(words)}"
        )
    events: List[CrashEvent] = []
    for i in range(0, len(words), CRASH_EVENT_WORDS):
        chunk = words[i : i + CRASH_EVENT_WORDS]
        kind = chunk[1]
        events.append(
            CrashEvent(
                seq=chunk[0],
                kind=kind,
                kind_label=EVENT_KIND_LABELS.get(kind, "unknown"),
                tick=chunk[2],
                cpu_id=chunk[3],
                task_id=chunk[4],
                reason_hash=f"0x{chunk[5]:x}",
                aux0=chunk[6],
                aux1=chunk[7],
            )
        )
    return events


def parse_log_events(lines: List[str]) -> List[CrashEvent]:
    events: List[CrashEvent] = []
    for line in lines:
        m = KERNEL_EVENT_RE.search(line)
        if not m:
            continue
        kind = int(m.group("kind"))
        events.append(
            CrashEvent(
                seq=int(m.group("seq")),
                kind=kind,
                kind_label=EVENT_KIND_LABELS.get(kind, "unknown"),
                tick=int(m.group("tick")),
                cpu_id=int(m.group("cpu")),
                task_id=int(m.group("task")),
                reason_hash=m.group("hash").lower(),
                aux0=int(m.group("aux0")),
                aux1=int(m.group("aux1")),
            )
        )
    return events


def parse_log_panic(lines: List[str]) -> Dict[str, object]:
    panic_reports = 0
    latest_reason = None
    latest_hash = None
    for line in lines:
        m = PANIC_RE.search(line)
        if m:
            panic_reports += 1
            latest_reason = m.group("reason")
            latest_hash = m.group("hash").lower()

    for line in reversed(lines):
        km = KERNEL_PANIC_RE.search(line)
        if km:
            return {
                "panic_count": int(km.group("count")),
                "last_panic_tick": int(km.group("tick")),
                "last_reason_hash": km.group("hash").lower(),
                "panic_reports_seen_in_log": panic_reports,
                "latest_panic_reason_from_report": latest_reason,
                "latest_panic_hash_from_report": latest_hash,
            }

    return {
        "panic_count": panic_reports,
        "last_panic_tick": 0,
        "last_reason_hash": latest_hash or "0x0",
        "panic_reports_seen_in_log": panic_reports,
        "latest_panic_reason_from_report": latest_reason,
        "latest_panic_hash_from_report": latest_hash,
    }


def summarize(events: List[CrashEvent]) -> Dict[str, int]:
    out: Dict[str, int] = {}
    for ev in events:
        out[ev.kind_label] = out.get(ev.kind_label, 0) + 1
    return out


def render_markdown(payload: Dict[str, object]) -> str:
    lines: List[str] = []
    lines.append("# Crash Artifact Report")
    lines.append("")

    source = payload.get("source", {})
    lines.append("## Source")
    lines.append("")
    lines.append(f"- Log file: `{source.get('log_path', '-')}`")
    lines.append(
        f"- Parsed events: **{payload.get('event_count', 0)}** (latest seq: **{payload.get('latest_seq', 0)}**)"
    )
    lines.append("")

    panic = payload.get("panic", {})
    lines.append("## Panic Snapshot")
    lines.append("")
    lines.append(f"- panic_count: `{panic.get('panic_count', 0)}`")
    lines.append(f"- last_panic_tick: `{panic.get('last_panic_tick', 0)}`")
    lines.append(f"- last_reason_hash: `{panic.get('last_reason_hash', '0x0')}`")
    if panic.get("latest_panic_reason_from_report"):
        lines.append(
            f"- latest_reason: `{panic.get('latest_panic_reason_from_report')}`"
        )
    lines.append("")

    crash_report = payload.get("decoded_crash_report")
    if crash_report:
        lines.append("## Decoded GET_CRASH_REPORT")
        lines.append("")
        for k, v in crash_report.items():
            lines.append(f"- {k}: `{v}`")
        lines.append("")

    hist = payload.get("event_histogram", {})
    lines.append("## Event Histogram")
    lines.append("")
    if hist:
        for k in sorted(hist.keys()):
            lines.append(f"- {k}: `{hist[k]}`")
    else:
        lines.append("- no crash events found")
    lines.append("")

    latest = payload.get("latest_event")
    lines.append("## Latest Event")
    lines.append("")
    if latest:
        for k, v in latest.items():
            lines.append(f"- {k}: `{v}`")
    else:
        lines.append("- none")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate crash artifact summary report")
    parser.add_argument(
        "--log",
        type=Path,
        default=None,
        help="Kernel log file path (serial output / captured dump)",
    )
    parser.add_argument(
        "--crash-report-words",
        default=None,
        help=(
            "Comma-separated usize words from GET_CRASH_REPORT (10 words). "
            "Example: 1,100,0xabc,200,2,1,5,0,7,1"
        ),
    )
    parser.add_argument(
        "--crash-events-words",
        default=None,
        help=(
            "Comma-separated usize words from LIST_CRASH_EVENTS. "
            "Each event is 8 words: seq,kind,tick,cpu,task,reason_hash,aux0,aux1"
        ),
    )
    parser.add_argument("--format", choices=("md", "json"), default="md")
    parser.add_argument("--out", type=Path, default=None, help="Output file path")
    args = parser.parse_args()

    lines: List[str] = []
    if args.log:
        lines = args.log.read_text(encoding="utf-8", errors="replace").splitlines()

    log_events = parse_log_events(lines)
    log_panic = parse_log_panic(lines)

    report_words = parse_word_list(args.crash_report_words)
    event_words = parse_word_list(args.crash_events_words)
    decoded_report = decode_crash_report_words(report_words)
    decoded_events = decode_crash_event_words(event_words)

    merged_events = log_events + decoded_events
    latest_event = max(merged_events, key=lambda e: e.seq) if merged_events else None

    payload = {
        "source": {
            "log_path": str(args.log) if args.log else "-",
            "crash_report_words_supplied": bool(report_words),
            "crash_events_words_supplied": bool(event_words),
        },
        "panic": log_panic,
        "event_count": len(merged_events),
        "latest_seq": latest_event.seq if latest_event else 0,
        "event_histogram": summarize(merged_events),
        "latest_event": asdict(latest_event) if latest_event else None,
        "decoded_crash_report": asdict(decoded_report) if decoded_report else None,
    }

    rendered = render_markdown(payload) if args.format == "md" else json.dumps(payload, indent=2)

    if args.out:
        out = args.out
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
