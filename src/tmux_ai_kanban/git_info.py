"""Git information extractor."""

import os
import subprocess
from typing import List, Optional

from .models import GitInfo, GitStatus


def run_git(args: List[str], cwd: str, check: bool = False) -> tuple:
    """Run git command in directory."""
    try:
        result = subprocess.run(
            ["git"] + args,
            cwd=cwd,
            capture_output=True,
            text=True,
            check=False
        )
        if check and result.returncode != 0:
            return result.returncode, ""
        return result.returncode, result.stdout.strip()
    except FileNotFoundError:
        return -1, ""


def is_git_repo(path: str) -> bool:
    """Check if path is a git repository."""
    code, _ = run_git(["rev-parse", "--git-dir"], cwd=path)
    return code == 0


def get_git_branch(path: str) -> Optional[str]:
    """Get current git branch."""
    code, output = run_git(["branch", "--show-current"], cwd=path)
    if code == 0 and output:
        return output
    
    # Try to get detached HEAD
    code, output = run_git(["rev-parse", "--short", "HEAD"], cwd=path)
    if code == 0:
        return f"({output})"
    return None


def get_git_commit(path: str) -> Optional[str]:
    """Get current git commit hash."""
    code, output = run_git(["rev-parse", "HEAD"], cwd=path)
    if code == 0:
        return output
    return None


def get_git_commit_msg(path: str) -> Optional[str]:
    """Get current commit message."""
    code, output = run_git(["log", "-1", "--pretty=%s"], cwd=path)
    if code == 0:
        return output
    return None


def get_changed_files_count(path: str) -> int:
    """Get number of changed files."""
    code, output = run_git(["status", "--porcelain"], cwd=path)
    if code == 0:
        return len([l for l in output.split("\n") if l.strip()])
    return 0


def get_git_info(path: str) -> Optional[GitInfo]:
    """Get complete git information for a path.
    
    Args:
        path: Directory path
    
    Returns:
        GitInfo object or None if not a git repo
    """
    if not os.path.isdir(path):
        return GitInfo(status=GitStatus.NO_GIT)
    
    if not is_git_repo(path):
        return GitInfo(status=GitStatus.NO_GIT)
    
    branch = get_git_branch(path)
    commit = get_git_commit(path)
    commit_msg = get_git_commit_msg(path)
    changed = get_changed_files_count(path)
    
    return GitInfo(
        branch=branch,
        commit=commit,
        commit_msg=commit_msg,
        status=GitStatus.DIRTY if changed > 0 else GitStatus.CLEAN,
        changed_files=changed,
    )
