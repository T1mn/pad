#!/usr/bin/env python3
"""Keep ordinary Rust source files small enough to review quickly."""

from __future__ import annotations

import subprocess
import sys

MAX_RUST_LINES = 200
EXEMPT_PREFIXES = (
    "rust-tui/src/i18n/",  # static translation tables are intentionally dense.
)


def tracked_rust_files() -> list[str]:
    output = subprocess.check_output(["git", "ls-files", "rust-tui/src"], text=True)
    return [line for line in output.splitlines() if line.endswith(".rs")]


def line_count(path: str) -> int:
    with open(path, "r", encoding="utf-8") as handle:
        return sum(1 for _ in handle)


def is_exempt(path: str) -> bool:
    return path.startswith(EXEMPT_PREFIXES)


def main() -> int:
    errors = []
    for path in tracked_rust_files():
        if is_exempt(path):
            continue
        lines = line_count(path)
        if lines > MAX_RUST_LINES:
            errors.append(f"rust file too long: {path} has {lines} lines > {MAX_RUST_LINES}")

    if errors:
        print("rust file size check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("rust file size check ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
