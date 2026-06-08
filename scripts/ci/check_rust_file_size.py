#!/usr/bin/env python3
"""Keep ordinary Rust source files small enough to review quickly."""

from __future__ import annotations

import subprocess
import sys

MAX_RUST_LINES = 200
INLINE_TEST_MARKER = "mod tests {"
EXEMPT_PREFIXES = (
    "rust-tui/src/i18n/",  # static translation tables are intentionally dense.
)


def tracked_rust_files() -> list[str]:
    output = subprocess.check_output(["git", "ls-files", "rust-tui/src"], text=True)
    return [line for line in output.splitlines() if line.endswith(".rs")]


def read_file(path: str) -> str:
    with open(path, "r", encoding="utf-8") as handle:
        return handle.read()


def line_count(text: str) -> int:
    return text.count("\n") + (0 if not text else 1)


def may_have_inline_tests(path: str) -> bool:
    return path.endswith("tests.rs") or "/tests/" in path


def is_exempt(path: str) -> bool:
    return path.startswith(EXEMPT_PREFIXES)


def main() -> int:
    errors = []
    for path in tracked_rust_files():
        text = read_file(path)
        if not is_exempt(path):
            lines = line_count(text)
            if lines > MAX_RUST_LINES:
                errors.append(f"rust file too long: {path} has {lines} lines > {MAX_RUST_LINES}")
        if not may_have_inline_tests(path) and INLINE_TEST_MARKER in text:
            errors.append(f"inline test module found: {path} should use an external *_tests.rs file")

    if errors:
        print("rust file size check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("rust file size check ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
