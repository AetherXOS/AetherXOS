#!/usr/bin/env python3
"""
Generate a lightweight P2 gap report from source markers and roadmap docs.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import re
from pathlib import Path
from typing import Dict, List, Tuple


MARKER_PATTERNS: List[Tuple[str, re.Pattern[str]]] = [
    ("todo", re.compile(r"\bTODO\b")),
    ("fixme", re.compile(r"\bFIXME\b")),
    ("mock", re.compile(r"\bmock\b", re.IGNORECASE)),
    ("stub", re.compile(r"\bstub\b", re.IGNORECASE)),
    ("unimplemented", re.compile(r"\bunimplemented!\b", re.IGNORECASE)),
    ("todo_macro", re.compile(r"\btodo!\b", re.IGNORECASE)),
]


def scan_file(path: Path) -> Dict[str, int]:
    counts = {name: 0 for name, _ in MARKER_PATTERNS}
    try:
        text = path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return counts
    ext = path.suffix.lower()
    for line in text.splitlines():
        is_comment_line = True
        if ext == ".rs":
            stripped = line.lstrip()
            is_comment_line = (
                stripped.startswith("//")
                or stripped.startswith("/*")
                or stripped.startswith("*")
            )
        if not is_comment_line:
            continue
        for name, pattern in MARKER_PATTERNS:
            counts[name] += len(pattern.findall(line))
    return counts


def merge_counts(dst: Dict[str, int], src: Dict[str, int]) -> None:
    for k, v in src.items():
        dst[k] = dst.get(k, 0) + int(v)


def is_excluded(rel: str, excludes: List[str]) -> bool:
    normalized = rel.replace("\\", "/")
    return any(fnmatch.fnmatch(normalized, pat) for pat in excludes)


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate P2 gap report")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("reports/p2_gap"),
        help="Output directory",
    )
    parser.add_argument(
        "--top-n",
        type=int,
        default=15,
        help="Top module hotspots to include",
    )
    parser.add_argument(
        "--exclude",
        action="append",
        default=[],
        help="Glob-style relative path exclusion (can be repeated)",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    targets = [root / "src", root / "scripts", root / "docs", root / "README.md"]
    files: List[Path] = []
    for target in targets:
        if target.is_file():
            files.append(target)
            continue
        if target.is_dir():
            files.extend(
                p
                for p in target.rglob("*")
                if p.is_file() and p.suffix.lower() in {".rs", ".py", ".ps1", ".md", ".toml"}
            )

    default_excludes = [
        "scripts/p2_gap_report.py",
        "scripts/p2_gap_gate.py",
        "scripts/syscall_coverage_report.py",
        "docs/linux_syscall_coverage_report.md",
        "README.md",
    ]
    excludes = default_excludes + list(args.exclude)

    totals = {name: 0 for name, _ in MARKER_PATTERNS}
    actionable_totals = {name: 0 for name, _ in MARKER_PATTERNS}
    by_module: Dict[str, Dict[str, int]] = {}
    actionable_by_module: Dict[str, Dict[str, int]] = {}
    for path in files:
        rel = path.relative_to(root).as_posix()
        module = rel.split("/", 2)[1] if rel.startswith("src/") and "/" in rel else rel.split("/", 1)[0]
        counts = scan_file(path)
        if sum(counts.values()) == 0:
            continue
        if module not in by_module:
            by_module[module] = {name: 0 for name, _ in MARKER_PATTERNS}
        merge_counts(by_module[module], counts)
        merge_counts(totals, counts)
        if is_excluded(rel, excludes):
            continue
        if module not in actionable_by_module:
            actionable_by_module[module] = {name: 0 for name, _ in MARKER_PATTERNS}
        merge_counts(actionable_by_module[module], counts)
        merge_counts(actionable_totals, counts)

    hotspots = sorted(
        (
            {
                "module": module,
                "total_markers": int(sum(counts.values())),
                "markers": counts,
            }
            for module, counts in by_module.items()
        ),
        key=lambda x: x["total_markers"],
        reverse=True,
    )

    actionable_hotspots = sorted(
        (
            {
                "module": module,
                "total_markers": int(sum(counts.values())),
                "markers": counts,
            }
            for module, counts in actionable_by_module.items()
        ),
        key=lambda x: x["total_markers"],
        reverse=True,
    )

    roadmap_status = (root / "docs" / "ROADMAP_STATUS.md").read_text(encoding="utf-8", errors="ignore")
    p2_started = "## P2" in roadmap_status and "Not started" not in roadmap_status

    summary = {
        "totals": totals,
        "total_markers": int(sum(totals.values())),
        "actionable_totals": actionable_totals,
        "actionable_total_markers": int(sum(actionable_totals.values())),
        "actionable_excludes": excludes,
        "top_hotspots": hotspots[: max(1, args.top_n)],
        "top_actionable_hotspots": actionable_hotspots[: max(1, args.top_n)],
        "p2_started_in_roadmap": p2_started,
    }
    payload = {"summary": summary}
    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")

    lines = [
        "# P2 Gap Report",
        "",
        f"- total_markers: `{summary['total_markers']}`",
        f"- actionable_total_markers: `{summary['actionable_total_markers']}`",
        f"- p2_started_in_roadmap: `{summary['p2_started_in_roadmap']}`",
        "",
        "## Marker Totals",
        "",
    ]
    for name, _ in MARKER_PATTERNS:
        lines.append(f"- `{name}`: {totals.get(name, 0)}")
    lines.extend(["", "## Actionable Marker Totals", ""])
    for name, _ in MARKER_PATTERNS:
        lines.append(f"- `{name}`: {actionable_totals.get(name, 0)}")
    lines.extend(["", "## Actionable Excludes", ""])
    for item in excludes:
        lines.append(f"- `{item}`")
    lines.extend(["", "## Top Hotspots", ""])
    for item in summary["top_hotspots"]:
        lines.append(f"- `{item['module']}` total={item['total_markers']} markers={item['markers']}")
    lines.extend(["", "## Top Actionable Hotspots", ""])
    for item in summary["top_actionable_hotspots"]:
        lines.append(f"- `{item['module']}` total={item['total_markers']} markers={item['markers']}")
    (out_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"p2 gap report written: {out_dir / 'summary.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
