#!/usr/bin/env python3
"""Keep the Rust harness compact while retaining consolidated case coverage."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path


EXPECTED_ENTRIES = 100
EXPECTED_DOMAIN_SUITES = 93
EXPECTED_SYNC_CASES = 769
EXPECTED_ASYNC_TESTS = 5
EXPECTED_IGNORED_TESTS = 2

TEST_ENTRY = re.compile(r"^[ \t]*#\[(?:test|tokio::test(?:\([^]]*\))?)\]", re.MULTILINE)
PLAIN_TEST = re.compile(r"^[ \t]*#\[test\]", re.MULTILINE)
ASYNC_TEST = re.compile(r"^[ \t]*#\[tokio::test(?:\([^]]*\))?\]", re.MULTILINE)
IGNORED_TEST = re.compile(r"^[ \t]*#\[ignore(?:\([^]]*\))?\]", re.MULTILINE)
SUITE_CASE = re.compile(r"\bcrate::[A-Za-z_][A-Za-z0-9_:]*(?=,)")


def workspace_rust_files() -> list[str]:
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "rust-tui/src"],
        text=True,
    )
    deleted = set(
        subprocess.check_output(
            ["git", "ls-files", "--deleted", "rust-tui/src"], text=True
        ).splitlines()
    )
    return [
        path
        for path in output.splitlines()
        if path.endswith(".rs") and path not in deleted
    ]


def count(pattern: re.Pattern[str], texts: list[str]) -> int:
    return sum(len(pattern.findall(text)) for text in texts)


def main() -> int:
    paths = workspace_rust_files()
    texts = [Path(path).read_text(encoding="utf-8") for path in paths]
    suite_texts = [
        Path(path).read_text(encoding="utf-8")
        for path in paths
        if Path(path).name.startswith("test_suites_")
    ]

    entries = count(TEST_ENTRY, texts)
    async_tests = count(ASYNC_TEST, texts)
    ignored_tests = count(IGNORED_TEST, texts)
    suite_entries = count(PLAIN_TEST, suite_texts)
    sync_cases = count(SUITE_CASE, suite_texts)
    expected = {
        "test entries": (entries, EXPECTED_ENTRIES),
        "domain suites": (suite_entries, EXPECTED_DOMAIN_SUITES),
        "suite case calls": (sync_cases, EXPECTED_SYNC_CASES),
        "async tests": (async_tests, EXPECTED_ASYNC_TESTS),
        "ignored tests": (ignored_tests, EXPECTED_IGNORED_TESTS),
    }
    errors = [
        f"{label}: found {actual}, expected {target}"
        for label, (actual, target) in expected.items()
        if actual != target
    ]
    if errors:
        print("test suite check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(
        f"test suite check ok: {entries} entries execute {sync_cases} sync cases, "
        f"plus {async_tests} async and {ignored_tests} ignored tests"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
