#!/usr/bin/env python3
"""
Linux ABI gap inventory reporter.

Scans Linux compatibility syscall surfaces and inventories likely runtime gaps:
- ENOSYS / linux_nosys() -> stub
- EOPNOTSUPP            -> partial_or_feature_gated

Outputs JSON and Markdown artifacts for CI visibility.
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Dict, List, Optional

TARGET_GLOBS = (
    "src/modules/linux_compat/**/*.rs",
    "src/kernel/syscalls/linux_shim/**/*.rs",
)

LINUX_COMPAT_ONLY_GLOBS = ("src/modules/linux_compat/**/*.rs",)

FN_DEF_RE = re.compile(
    r"(?:pub(?:\([^)]+\))?\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(",
    re.M,
)

TOKEN_PATTERNS = {
    "stub": (
        "linux_nosys()",
        "errno::ENOSYS",
        "linux_errno(linux::ENOSYS)",
        "linux_errno(crate::modules::posix_consts::errno::ENOSYS)",
    ),
    "partial_or_feature_gated": (
        "errno::EOPNOTSUPP",
        "linux_errno(linux::EOPNOTSUPP)",
        "linux_errno(crate::modules::posix_consts::errno::EOPNOTSUPP)",
    ),
}


@dataclass
class GapEntry:
    category: str
    token: str
    file: str
    line: int
    function: str


@dataclass
class Summary:
    scanned_files: int
    total_gaps: int
    stub_count: int
    partial_or_feature_gated_count: int


def extract_fn_body(text: str, fn_start: int) -> str:
    brace_open = text.find("{", fn_start)
    if brace_open < 0:
        return ""
    depth = 0
    i = brace_open
    n = len(text)
    while i < n:
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[brace_open : i + 1]
        i += 1
    return text[brace_open:]


def find_function_ranges(text: str) -> List[tuple[str, int, int]]:
    ranges: List[tuple[str, int, int]] = []
    starts: List[tuple[str, int]] = []
    for m in FN_DEF_RE.finditer(text):
        fn_name = m.group(1)
        starts.append((fn_name, m.start()))

    for fn_name, start in starts:
        body = extract_fn_body(text, start)
        if not body:
            continue
        body_start = text.find(body, start)
        body_end = body_start + len(body)
        ranges.append((fn_name, body_start, body_end))
    return ranges


def locate_function(function_ranges: List[tuple[str, int, int]], offset: int) -> str:
    for fn_name, start, end in function_ranges:
        if start <= offset <= end:
            return fn_name
    return "<module_scope>"


def offset_to_line(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def collect_entries_for_file(repo_root: Path, file_path: Path) -> List[GapEntry]:
    text = file_path.read_text(encoding="utf-8", errors="replace")
    function_ranges = find_function_ranges(text)
    relative_file = str(file_path.relative_to(repo_root)).replace("\\", "/")

    entries: List[GapEntry] = []
    for category, tokens in TOKEN_PATTERNS.items():
        for token in tokens:
            start = 0
            while True:
                idx = text.find(token, start)
                if idx < 0:
                    break
                entries.append(
                    GapEntry(
                        category=category,
                        token=token,
                        file=relative_file,
                        line=offset_to_line(text, idx),
                        function=locate_function(function_ranges, idx),
                    )
                )
                start = idx + len(token)

    return entries


def dedupe(entries: List[GapEntry]) -> List[GapEntry]:
    seen: set[tuple[str, str, int, str]] = set()
    out: List[GapEntry] = []
    for e in sorted(entries, key=lambda x: (x.file, x.line, x.category, x.token)):
        key = (e.file, e.category, e.line, e.function)
        if key in seen:
            continue
        seen.add(key)
        out.append(e)
    return out


def filter_non_runtime_noise(entries: List[GapEntry]) -> List[GapEntry]:
    out: List[GapEntry] = []
    for e in entries:
        fn = e.function
        if fn == "<module_scope>" or fn in {"linux_nosys", "no_sys", "sys_linux_shim"}:
            continue
        if fn.startswith("test_") or fn.startswith("syscall_negative_paths_"):
            continue
        if "_returns_" in fn or "_behavior" in fn:
            continue
        out.append(e)
    return out


def to_markdown(summary: Summary, entries: List[GapEntry]) -> str:
    lines: List[str] = []
    lines.append("# Linux ABI Gap Inventory")
    lines.append("")
    lines.append(f"- scanned_files: {summary.scanned_files}")
    lines.append(f"- total_gaps: {summary.total_gaps}")
    lines.append(f"- stub_count: {summary.stub_count}")
    lines.append(
        f"- partial_or_feature_gated_count: {summary.partial_or_feature_gated_count}"
    )
    lines.append("")
    lines.append("| Category | Function | File | Line | Token |")
    lines.append("|---|---|---|---:|---|")
    for e in entries:
        lines.append(
            f"| {e.category} | {e.function} | {e.file} | {e.line} | {e.token} |"
        )
    lines.append("")
    return "\n".join(lines)


def build_payload(summary: Summary, entries: List[GapEntry]) -> Dict[str, object]:
    return {
        "summary": asdict(summary),
        "entries": [asdict(e) for e in entries],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate Linux ABI gap inventory")
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("reports/abi_gap_inventory"),
        help="Output directory for summary artifacts",
    )
    parser.add_argument(
        "--max-stub",
        type=int,
        default=None,
        help="Fail if stub count exceeds this value",
    )
    parser.add_argument(
        "--max-partial",
        type=int,
        default=None,
        help="Fail if partial_or_feature_gated count exceeds this value",
    )
    parser.add_argument(
        "--linux-compat-only",
        action="store_true",
        help="Scan only src/modules/linux_compat (exclude kernel linux_shim tree)",
    )
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    selected_globs = LINUX_COMPAT_ONLY_GLOBS if args.linux_compat_only else TARGET_GLOBS

    files: List[Path] = []
    for glob_pat in selected_globs:
        files.extend(root.glob(glob_pat))

    all_entries: List[GapEntry] = []
    scanned_files = 0
    for file_path in sorted(set(files)):
        if not file_path.is_file():
            continue
        scanned_files += 1
        all_entries.extend(collect_entries_for_file(root, file_path))

    entries = dedupe(all_entries)
    entries = filter_non_runtime_noise(entries)
    stub_count = sum(1 for e in entries if e.category == "stub")
    partial_count = sum(1 for e in entries if e.category == "partial_or_feature_gated")

    summary = Summary(
        scanned_files=scanned_files,
        total_gaps=len(entries),
        stub_count=stub_count,
        partial_or_feature_gated_count=partial_count,
    )

    payload = build_payload(summary, entries)
    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
    (out_dir / "summary.md").write_text(to_markdown(summary, entries), encoding="utf-8")

    print("linux abi gap inventory: PASS")
    print(f"summary={out_dir / 'summary.json'}")

    failures: List[str] = []
    if args.max_stub is not None and stub_count > args.max_stub:
        failures.append(f"stub_count={stub_count} > max_stub={args.max_stub}")
    if args.max_partial is not None and partial_count > args.max_partial:
        failures.append(f"partial_count={partial_count} > max_partial={args.max_partial}")

    if failures:
        print("linux abi gap inventory gate: FAIL")
        for failure in failures:
            print(f"- {failure}")
        return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
