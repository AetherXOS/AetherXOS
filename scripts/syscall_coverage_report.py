#!/usr/bin/env python3
"""
Linux syscall coverage reporter for this kernel tree.

Scans:
- src/modules/linux_compat/sys_dispatcher/**/*.rs  (syscall -> handler mapping)
- src/modules/linux_compat/**/*.rs                 (handler definitions)
- src/kernel/syscalls/mod.rs                       (linux_nr dispatch fallback)

Classifies mapped handlers into:
- implemented
- partial
- no
- external
"""

from __future__ import annotations

import argparse
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple


DISPATCH_GLOB = "src/modules/linux_compat/sys_dispatcher/**/*.rs"
HANDLER_GLOB = "src/modules/linux_compat/**/*.rs"
KERNEL_DISPATCH_FILE = "src/kernel/syscalls/mod.rs"
KERNEL_SYSCALL_FILE = "src/kernel/syscalls/mod.rs"

MAP_RE = re.compile(
    r"(?:(?:#\s*\[\s*cfg\s*\((.*?)\)\s*\]\s*)?)linux_nr::([A-Z0-9_]+)\s*=>\s*Some\((.*?)\),",
    re.S,
)
FN_NAME_RE = re.compile(r"\b(sys_linux_[a-zA-Z0-9_]+)\b")
FN_DEF_RE = re.compile(r"\b(?:pub(?:\(crate\))?\s+)?fn\s+(sys_linux_[a-zA-Z0-9_]+)\s*\(")
CFG_NOT_RE = re.compile(r"#\s*\[\s*cfg\s*\(\s*not\s*\(")
LINE_COMMENT_RE = re.compile(r"//.*?$", re.M)
BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/", re.S)
ALWAYS_ERR_RE = re.compile(r"^\{\s*(?:linux_(?:eperm|inval|nosys)\(\)\s*;?\s*)\}$", re.S)


@dataclass
class HandlerDef:
    name: str
    file: Path
    body: str


@dataclass
class SyscallRow:
    nr_name: str
    handler: str
    status: str
    file: str
    reason: str


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


def _cfg_allows(condition: str, linux_compat_enabled: bool) -> bool:
    cond = " ".join(condition.split()).lower()
    if not cond:
        return True
    if 'feature = "linux_compat"' in cond and "not(" not in cond:
        return linux_compat_enabled
    if 'not(feature = "linux_compat")' in cond:
        return not linux_compat_enabled
    return True


def _collect_mappings_from_text(
    text: str, out: Dict[str, str], linux_compat_enabled: bool
) -> None:
    for m in MAP_RE.finditer(text):
        cond = (m.group(1) or "").strip()
        if not _cfg_allows(cond, linux_compat_enabled):
            continue
        nr = m.group(2)
        expr = m.group(3)
        fns = FN_NAME_RE.findall(expr)
        if fns:
            out[nr] = fns[-1]
        else:
            compact_expr = " ".join(expr.split())
            out[nr] = f"<expr>{compact_expr}"


def collect_mappings(root: Path, linux_compat_enabled: bool) -> Dict[str, str]:
    out: Dict[str, str] = {}
    for file in root.glob(DISPATCH_GLOB):
        text = file.read_text(encoding="utf-8", errors="replace")
        _collect_mappings_from_text(text, out, linux_compat_enabled)
    kernel_dispatch = root / KERNEL_DISPATCH_FILE
    if kernel_dispatch.exists():
        text = kernel_dispatch.read_text(encoding="utf-8", errors="replace")
        _collect_mappings_from_text(text, out, linux_compat_enabled)
    return out


def collect_handlers(root: Path) -> Dict[str, HandlerDef]:
    out: Dict[str, HandlerDef] = {}
    for file in root.glob(HANDLER_GLOB):
        text = file.read_text(encoding="utf-8", errors="replace")
        for m in FN_DEF_RE.finditer(text):
            fn = m.group(1)
            body = extract_fn_body(text, m.start())
            out[fn] = HandlerDef(name=fn, file=file, body=body)
    kernel_file = root / KERNEL_SYSCALL_FILE
    if kernel_file.exists():
        text = kernel_file.read_text(encoding="utf-8", errors="replace")
        for m in FN_DEF_RE.finditer(text):
            fn = m.group(1)
            body = extract_fn_body(text, m.start())
            out[fn] = HandlerDef(name=fn, file=kernel_file, body=body)
    return out


def strip_cfg_not_blocks(text: str) -> str:
    lines = text.splitlines(keepends=True)
    out: List[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if CFG_NOT_RE.search(line):
            j = i + 1
            while j < len(lines) and lines[j].strip() == "":
                j += 1
            if j < len(lines) and "{" in lines[j]:
                depth = 0
                started = False
                k = j
                while k < len(lines):
                    for ch in lines[k]:
                        if ch == "{":
                            depth += 1
                            started = True
                        elif ch == "}":
                            depth -= 1
                    k += 1
                    if started and depth <= 0:
                        break
                i = k
                continue
        out.append(line)
        i += 1
    return "".join(out)


def normalize_for_classification(body: str) -> str:
    body = strip_cfg_not_blocks(body)
    body = BLOCK_COMMENT_RE.sub("", body)
    body = LINE_COMMENT_RE.sub("", body)
    return body.lower()


def classify(handler: Optional[HandlerDef], handler_name: str) -> Tuple[str, str, str]:
    if handler_name.startswith("<expr>"):
        expr = handler_name[len("<expr>") :].lower()
        if "linux_nosys()" in expr or "enosys" in expr:
            return ("no", "-", "direct expression returns ENOSYS")
        if "eopnotsupp" in expr:
            return ("partial", "-", "direct expression returns EOPNOTSUPP")
        if "sys_" in expr or expr == "arg1":
            return ("implemented", "-", "direct expression resolves to core syscall path")
        return ("external", "-", "mapped to non-linux_compat expression")
    if handler is None:
        return ("external", "-", "handler definition not found")

    raw_lc = handler.body.lower()
    body_lc = normalize_for_classification(handler.body)

    if "linux_nosys()" in body_lc:
        return ("no", str(handler.file).replace("\\", "/"), "contains linux_nosys()")

    if "eopnotsupp" in body_lc or "enosys" in body_lc:
        return ("partial", str(handler.file).replace("\\", "/"), "returns EOPNOTSUPP/ENOSYS path")

    if "todo" in raw_lc or "mock" in raw_lc or "stub" in raw_lc or "no-op" in raw_lc:
        return ("partial", str(handler.file).replace("\\", "/"), "contains TODO/mock/stub/no-op markers")

    if "partial implementation" in raw_lc:
        return ("partial", str(handler.file).replace("\\", "/"), "contains partial implementation marker")

    if ALWAYS_ERR_RE.match(handler.body.strip()):
        return ("partial", str(handler.file).replace("\\", "/"), "always returns fixed policy error")

    return ("implemented", str(handler.file).replace("\\", "/"), "no unsupported markers detected")


def build_rows(root: Path, linux_compat_enabled: bool) -> List[SyscallRow]:
    mappings = collect_mappings(root, linux_compat_enabled)
    handlers = collect_handlers(root)
    rows: List[SyscallRow] = []
    for nr_name, handler_name in sorted(mappings.items(), key=lambda kv: kv[0]):
        status, file, reason = classify(handlers.get(handler_name), handler_name)
        rows.append(
            SyscallRow(
                nr_name=nr_name,
                handler=handler_name,
                status=status,
                file=file,
                reason=reason,
            )
        )
    return rows


def to_markdown(rows: List[SyscallRow]) -> str:
    total = len(rows)
    counts = {"implemented": 0, "partial": 0, "no": 0, "external": 0}
    for r in rows:
        counts[r.status] = counts.get(r.status, 0) + 1

    def pct(v: int) -> str:
        return f"{(100.0 * v / total):.1f}%" if total else "0.0%"

    lines = []
    lines.append("# Linux Syscall Coverage Report")
    lines.append("")
    lines.append(f"Total mapped syscalls: **{total}**")
    lines.append("")
    lines.append("| Status | Count | Percent |")
    lines.append("|---|---:|---:|")
    for k in ("implemented", "partial", "no", "external"):
        lines.append(f"| {k} | {counts[k]} | {pct(counts[k])} |")
    lines.append("")
    lines.append("| Linux NR | Handler | Status | File | Reason |")
    lines.append("|---|---|---|---|---|")
    for r in rows:
        lines.append(f"| {r.nr_name} | `{r.handler}` | {r.status} | `{r.file}` | {r.reason} |")
    lines.append("")
    return "\n".join(lines)


def to_json(rows: List[SyscallRow]) -> str:
    return json.dumps(
        [
            {
                "linux_nr": r.nr_name,
                "handler": r.handler,
                "status": r.status,
                "file": r.file,
                "reason": r.reason,
            }
            for r in rows
        ],
        indent=2,
    )


def summarize(rows: List[SyscallRow]) -> Dict[str, float]:
    total = len(rows)
    counts = {"implemented": 0, "partial": 0, "no": 0, "external": 0}
    for r in rows:
        counts[r.status] = counts.get(r.status, 0) + 1
    implemented_pct = (100.0 * counts["implemented"] / total) if total else 0.0
    return {
        "total": total,
        "implemented": counts["implemented"],
        "partial": counts["partial"],
        "no": counts["no"],
        "external": counts["external"],
        "implemented_pct": implemented_pct,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate Linux syscall coverage report")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1], help="Repository root")
    parser.add_argument("--format", choices=("md", "json"), default="md", help="Output format")
    parser.add_argument("--out", type=Path, default=None, help="Write report to file")
    parser.add_argument(
        "--linux-compat-enabled",
        action="store_true",
        help="Evaluate cfg(feature=\"linux_compat\") mappings as active",
    )
    parser.add_argument(
        "--summary-out",
        type=Path,
        default=None,
        help="Write summary JSON (counts and percentages) to file",
    )
    parser.add_argument(
        "--min-implemented-pct",
        type=float,
        default=None,
        help="Fail if implemented percentage is below this value",
    )
    parser.add_argument(
        "--max-no",
        type=int,
        default=None,
        help="Fail if 'no' syscall count is above this value",
    )
    parser.add_argument(
        "--max-partial",
        type=int,
        default=None,
        help="Fail if 'partial' syscall count is above this value",
    )
    parser.add_argument(
        "--max-external",
        type=int,
        default=None,
        help="Fail if 'external' syscall count is above this value",
    )
    args = parser.parse_args()

    rows = build_rows(args.root, args.linux_compat_enabled)
    summary = summarize(rows)
    rendered = to_markdown(rows) if args.format == "md" else to_json(rows)

    if args.out:
        out_path = args.out
        if not out_path.is_absolute():
            out_path = args.root / out_path
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(rendered, encoding="utf-8")
    else:
        print(rendered)

    if args.summary_out:
        summary_out = args.summary_out
        if not summary_out.is_absolute():
            summary_out = args.root / summary_out
        summary_out.parent.mkdir(parents=True, exist_ok=True)
        summary_out.write_text(json.dumps(summary, indent=2), encoding="utf-8")

    failures: List[str] = []
    if args.min_implemented_pct is not None and summary["implemented_pct"] < args.min_implemented_pct:
        failures.append(
            f"implemented_pct={summary['implemented_pct']:.2f} < min={args.min_implemented_pct:.2f}"
        )
    if args.max_no is not None and summary["no"] > args.max_no:
        failures.append(f"no={summary['no']} > max_no={args.max_no}")
    if args.max_partial is not None and summary["partial"] > args.max_partial:
        failures.append(f"partial={summary['partial']} > max_partial={args.max_partial}")
    if args.max_external is not None and summary["external"] > args.max_external:
        failures.append(f"external={summary['external']} > max_external={args.max_external}")

    if failures:
        print("syscall coverage gate: FAIL")
        for msg in failures:
            print(f"- {msg}")
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
