#!/usr/bin/env python3
"""Validate repository index.md conventions."""

from __future__ import annotations

import subprocess
import sys
from pathlib import PurePosixPath

MAX_INDEX_LINES = 50


def tracked_files() -> set[str]:
    output = subprocess.check_output(["git", "ls-files"], text=True)
    return {line for line in output.splitlines() if line}


def tracked_dirs(files: set[str]) -> set[str]:
    dirs: set[str] = {"."}
    for file in files:
        path = PurePosixPath(file)
        for parent in path.parents:
            if str(parent) == ".":
                dirs.add(".")
                break
            dirs.add(str(parent))
    return dirs


def index_path(directory: str) -> str:
    return "index.md" if directory == "." else f"{directory}/index.md"


def line_count(path: str) -> int:
    with open(path, "r", encoding="utf-8") as handle:
        return sum(1 for _ in handle)


def main() -> int:
    files = tracked_files()
    errors: list[str] = []

    for directory in sorted(tracked_dirs(files)):
        expected = index_path(directory)
        if expected not in files:
            errors.append(f"missing index: {expected}")

    for file in sorted(path for path in files if path.endswith("index.md")):
        lines = line_count(file)
        if lines > MAX_INDEX_LINES:
            errors.append(f"index too long: {file} has {lines} lines > {MAX_INDEX_LINES}")

    if errors:
        print("index check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("index check ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
