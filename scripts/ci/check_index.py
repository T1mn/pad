#!/usr/bin/env python3
"""Validate repository index.md conventions."""

from __future__ import annotations

import posixpath
import re
import subprocess
import sys
from pathlib import Path, PurePosixPath
from fnmatch import fnmatch

MAX_INDEX_LINES = 50
LOCAL_REF_SUFFIXES = (".rs", ".py", ".sh", ".md", ".yml", ".yaml")
LOCAL_GLOB_SUFFIXES = LOCAL_REF_SUFFIXES


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


def local_index_refs(index_file: str) -> tuple[list[str], list[str]]:
    text = Path(index_file).read_text(encoding="utf-8")
    refs: list[str] = []
    globs: list[str] = []
    for match in re.finditer(r"`([^`]+)`", text):
        ref = match.group(1).strip()
        if not ref or ref.startswith(("~", "/")):
            continue
        if any(token in ref for token in ("::", " ")):
            continue
        if "*" in ref:
            if ref.endswith(LOCAL_GLOB_SUFFIXES) and not ref.startswith("/"):
                globs.append(ref)
            continue
        if ref.startswith(("./", "../")) or ref.endswith("/") or ref.endswith(LOCAL_REF_SUFFIXES):
            refs.append(ref)
    return refs, globs


def validate_index_refs(index_file: str, files: set[str], dirs: set[str]) -> list[str]:
    base = PurePosixPath(index_file).parent
    if str(base) == ".":
        base = PurePosixPath("")
    errors: list[str] = []
    refs, globs = local_index_refs(index_file)
    for ref in refs:
        target = base / ref.rstrip("/")
        target_text = posixpath.normpath(str(target))
        if ref.endswith("/"):
            if target_text not in dirs:
                errors.append(f"stale index ref: {index_file} -> `{ref}`")
        elif target_text not in files:
            errors.append(f"stale index ref: {index_file} -> `{ref}`")
    for pattern in globs:
        target_pattern = posixpath.normpath(str(base / pattern))
        if not any(fnmatch(file, target_pattern) for file in files):
            errors.append(f"stale index glob: {index_file} -> `{pattern}`")
    return errors


def main() -> int:
    files = tracked_files()
    dirs = tracked_dirs(files)
    errors: list[str] = []

    for directory in sorted(dirs):
        expected = index_path(directory)
        if expected not in files:
            errors.append(f"missing index: {expected}")

    for file in sorted(path for path in files if path.endswith("index.md")):
        lines = line_count(file)
        if lines > MAX_INDEX_LINES:
            errors.append(f"index too long: {file} has {lines} lines > {MAX_INDEX_LINES}")
        errors.extend(validate_index_refs(file, files, dirs))

    if errors:
        print("index check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print("index check ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
