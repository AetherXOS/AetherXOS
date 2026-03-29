#!/usr/bin/env python3
"""
Run POSIX deep test gate and emit machine-readable summary artifacts.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import List


@dataclass
class StepResult:
    name: str
    ok: bool
    return_code: int
    duration_sec: float
    stdout_tail: str
    stderr_tail: str


def run_cmd(cmd: List[str], cwd: Path, timeout_sec: int) -> StepResult:
    start = time.perf_counter()
    try:
        proc = subprocess.run(
            cmd,
            cwd=str(cwd),
            capture_output=True,
            text=True,
            timeout=timeout_sec,
            check=False,
        )
        dur = time.perf_counter() - start
        return StepResult(
            name=" ".join(cmd),
            ok=proc.returncode == 0,
            return_code=proc.returncode,
            duration_sec=dur,
            stdout_tail="\n".join(proc.stdout.splitlines()[-40:]),
            stderr_tail="\n".join(proc.stderr.splitlines()[-40:]),
        )
    except subprocess.TimeoutExpired as exc:
        dur = time.perf_counter() - start
        return StepResult(
            name=" ".join(cmd),
            ok=False,
            return_code=124,
            duration_sec=dur,
            stdout_tail="\n".join((exc.stdout or "").splitlines()[-40:]),
            stderr_tail="\n".join((exc.stderr or "").splitlines()[-40:]),
        )


def discover_deep_tests(root: Path) -> int:
    deep_dir = root / "src" / "modules" / "posix" / "tests_deep"
    if not deep_dir.exists():
        return 0
    count = 0
    pattern = re.compile(r"^\s*#\[\s*(test|test_case)\s*\]\s*$")
    for file in sorted(deep_dir.rglob("*.rs")):
        for line in file.read_text(encoding="utf-8").splitlines():
            if pattern.match(line):
                count += 1
    return count


def main() -> int:
    parser = argparse.ArgumentParser(description="POSIX deep tests conformance gate")
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--timeout-sec", type=int, default=600)
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("reports/posix_conformance"),
    )
    args = parser.parse_args()

    root = args.root.resolve()
    out_dir = args.out_dir if args.out_dir.is_absolute() else root / args.out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    steps = []
    steps.append(
        run_cmd(
            ["cargo", "test", "--no-run", "--features", "posix_deep_tests"],
            root,
            args.timeout_sec,
        )
    )
    steps.append(
        run_cmd(
            ["cargo", "test", "--no-run", "--features", "posix_deep_tests", "--lib", "--tests"],
            root,
            args.timeout_sec,
        )
    )

    deep_test_count = discover_deep_tests(root)
    steps.append(
        StepResult(
            name="discover posix deep #[test] cases",
            ok=deep_test_count > 0,
            return_code=0 if deep_test_count > 0 else 1,
            duration_sec=0.0,
            stdout_tail=f"deep_test_count={deep_test_count}",
            stderr_tail="",
        )
    )

    ok = all(step.ok for step in steps)
    payload = {
        "summary": {
            "ok": ok,
            "steps": len(steps),
            "failed_steps": sum(1 for s in steps if not s.ok),
            "deep_test_count": deep_test_count,
        },
        "steps": [asdict(s) for s in steps],
    }
    (out_dir / "summary.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")

    lines = [
        "# POSIX Conformance Gate",
        "",
        f"- ok: `{ok}`",
        f"- steps: `{payload['summary']['steps']}`",
        f"- failed_steps: `{payload['summary']['failed_steps']}`",
        f"- deep_test_count: `{payload['summary']['deep_test_count']}`",
        "",
        "## Steps",
        "",
    ]
    for step in steps:
        lines.append(
            f"- `{step.name}` => ok `{step.ok}` rc `{step.return_code}` duration `{step.duration_sec:.2f}s`"
        )
    (out_dir / "summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"posix conformance gate: {'PASS' if ok else 'FAIL'}")
    print(f"summary={out_dir / 'summary.json'}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
