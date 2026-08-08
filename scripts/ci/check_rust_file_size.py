#!/usr/bin/env python3
"""Keep consolidated Rust modules cohesive without creating giant files."""

from __future__ import annotations

import subprocess
import sys

MAX_MODULE_LINES = 800
ABSOLUTE_MAX_LINES = 1000
EXEMPT_PREFIXES = (
    "rust-tui/src/i18n/",  # static translation tables are intentionally dense.
)


def workspace_rust_files() -> list[str]:
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "rust-tui/src"],
        text=True,
    )
    deleted = subprocess.check_output(
        ["git", "ls-files", "--deleted", "rust-tui/src"], text=True
    )
    deleted_paths = set(deleted.splitlines())
    return [
        line
        for line in output.splitlines()
        if line.endswith(".rs") and line not in deleted_paths
    ]


def read_file(path: str) -> str:
    with open(path, "r", encoding="utf-8") as handle:
        return handle.read()


def line_count(text: str) -> int:
    return text.count("\n") + (0 if not text else 1)


def is_exempt(path: str) -> bool:
    return path.startswith(EXEMPT_PREFIXES)


def main() -> int:
    errors = []
    for path in workspace_rust_files():
        text = read_file(path)
        lines = line_count(text)
        if lines > ABSOLUTE_MAX_LINES:
            errors.append(
                f"rust file exceeds absolute limit: {path} has {lines} lines "
                f"> {ABSOLUTE_MAX_LINES}"
            )
        elif not is_exempt(path) and lines > MAX_MODULE_LINES:
            errors.append(
                f"rust module too long: {path} has {lines} lines > {MAX_MODULE_LINES}"
            )

    if errors:
        print("rust file size check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("rust file size check ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
