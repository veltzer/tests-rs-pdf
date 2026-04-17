#!/usr/bin/env python
"""Build the comparison binary, run it on a sample SVG, and print a table
of results sorted by wall time (ascending).

Usage: run.py [path/to/input.svg]
"""
from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DEFAULT_SAMPLE = ROOT / "samples" / "the_tcp_ip_protocol_stack.svg"
OUT_DIR = ROOT / "out"
BINARY = ROOT / "target" / "release" / "compare"

ROW_RE = re.compile(
    r"^(?P<tool>\S.*?)\s{2,}(?P<ms>\d+)\s+(?P<bytes>\d+)\s+(?P<status>.*)$"
)


def main() -> int:
    sample = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_SAMPLE
    if not sample.exists():
        print(f"sample not found: {sample}", file=sys.stderr)
        return 2

    subprocess.run(
        ["cargo", "build", "--release", "--quiet"], cwd=ROOT, check=True
    )

    if OUT_DIR.exists():
        shutil.rmtree(OUT_DIR)
    OUT_DIR.mkdir(parents=True)

    result = subprocess.run(
        [str(BINARY), str(sample), str(OUT_DIR)],
        capture_output=True,
        text=True,
        check=True,
    )
    print(result.stdout, end="")

    rows = []
    for line in result.stdout.splitlines():
        m = ROW_RE.match(line)
        if not m:
            continue
        tool = m.group("tool").strip()
        if tool in {"tool", ""} or tool.startswith("----"):
            continue
        rows.append(
            {
                "tool": tool,
                "ms": int(m.group("ms")),
                "bytes": int(m.group("bytes")),
                "status": m.group("status"),
            }
        )

    rows.sort(key=lambda r: r["ms"])

    print()
    print("Sorted by speed:")
    print(f"{'tool':<20} {'wall_ms':>10} {'pdf_bytes':>12}")
    print("-" * 42)
    for r in rows:
        print(f"{r['tool']:<20} {r['ms']:>10} {r['bytes']:>12}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
