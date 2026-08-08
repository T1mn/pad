#!/usr/bin/env python3
"""Keep Rust modules cohesive without allowing unbounded source files."""

from __future__ import annotations

import subprocess
import sys

MAX_PRODUCTION_LINES = 500
MAX_TEST_LINES = 800
ABSOLUTE_MAX_LINES = 1000
INLINE_TEST_MARKER = "mod tests {"
EXEMPT_PREFIXES = (
    "rust-tui/src/i18n/",  # static translation tables are intentionally dense.
)


def workspace_rust_files() -> list[str]:
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "rust-tui/src"],
        text=True,
    )
    return [line for line in output.splitlines() if line.endswith(".rs")]


def read_file(path: str) -> str:
    with open(path, "r", encoding="utf-8") as handle:
        return handle.read()


def line_count(text: str) -> int:
    return text.count("\n") + (0 if not text else 1)


def is_test_file(path: str) -> bool:
    parts = path.split("/")
    return path.endswith("tests.rs") or any(
        part == "tests" or part.endswith("_tests") for part in parts[:-1]
    )


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
        elif not is_exempt(path):
            limit = MAX_TEST_LINES if is_test_file(path) else MAX_PRODUCTION_LINES
            if lines > limit:
                kind = "test" if is_test_file(path) else "production"
                errors.append(
                    f"rust {kind} file too long: {path} has {lines} lines > {limit}"
                )
        if not is_test_file(path) and INLINE_TEST_MARKER in text:
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
