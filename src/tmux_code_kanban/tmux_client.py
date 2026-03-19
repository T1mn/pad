"""Tmux client for executing tmux commands."""

import subprocess
from typing import List, Optional, Tuple


class TmuxError(Exception):
    """Tmux command error."""
    pass


def run_tmux(args: List[str], check: bool = True) -> Tuple[int, str, str]:
    """Run a tmux command and return (returncode, stdout, stderr)."""
    cmd = ["tmux"] + args
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=False
        )
        if check and result.returncode != 0:
            raise TmuxError(f"tmux command failed: {' '.join(cmd)}\n{result.stderr}")
        return result.returncode, result.stdout, result.stderr
    except FileNotFoundError:
        raise TmuxError("tmux not found in PATH")


def list_panes() -> List[dict]:
    """List all panes with their metadata.
    
    Returns list of dicts with keys:
    - session_name
    - window_name
    - pane_index
    - pane_id (e.g., %0)
    - pane_pid
    - pane_current_command
    - pane_current_path
    - pane_active
    """
    format_str = (
        "#{session_name}|"
        "#{window_name}|"
        "#{pane_index}|"
        "#{pane_id}|"
        "#{pane_pid}|"
        "#{pane_current_command}|"
        "#{pane_current_path}|"
        "#{pane_active}"
    )
    
    _, stdout, _ = run_tmux(["list-panes", "-a", "-F", format_str])
    
    panes = []
    for line in stdout.strip().split("\n"):
        if not line:
            continue
        parts = line.split("|")
        if len(parts) >= 8:
            panes.append({
                "session_name": parts[0],
                "window_name": parts[1],
                "pane_index": parts[2],
                "pane_id": parts[3],
                "pane_pid": int(parts[4]) if parts[4].isdigit() else 0,
                "pane_current_command": parts[5],
                "pane_current_path": parts[6],
                "pane_active": parts[7] == "1",
            })
    
    return panes


def capture_pane(pane_id: str, lines: int = 20, start_line: Optional[int] = None) -> str:
    """Capture content from a pane.
    
    Args:
        pane_id: Tmux pane id (e.g., %0)
        lines: Number of lines to capture from the end
        start_line: Start from specific line (negative for history), None for visible only
    
    Returns:
        Captured content
    """
    try:
        args = ["capture-pane", "-p", "-t", pane_id]
        if start_line is not None:
            args.extend(["-S", str(start_line)])
        if lines > 0:
            args.extend(["-E", str(lines) if start_line is None else str(start_line + lines)])
        
        _, stdout, _ = run_tmux(args, check=False)
        return stdout
    except TmuxError:
        return ""


def get_pane_history_size(pane_id: str) -> int:
    """Get total history lines available in pane."""
    try:
        _, stdout, _ = run_tmux(
            ["list-panes", "-t", pane_id, "-F", "#{history_size}"],
            check=False
        )
        return int(stdout.strip()) if stdout.strip().isdigit() else 0
    except (TmuxError, ValueError):
        return 0


def get_process_tree(pid: int) -> List[str]:
    """Get process tree starting from pid.
    
    Returns list of process command names.
    """
    commands = []
    
    # Try using pstree first
    try:
        result = subprocess.run(
            ["pstree", "-p", str(pid)],
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0:
            # Parse pstree output to extract process names
            output = result.stdout
            # Extract process names (text before parentheses)
            import re
            cmds = re.findall(r'([\w\-]+)\(', output)
            return cmds
    except FileNotFoundError:
        pass
    
    # Fallback: use ps
    try:
        result = subprocess.run(
            ["ps", "--ppid", str(pid), "-o", "comm="],
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0:
            commands = [line.strip() for line in result.stdout.strip().split("\n") if line.strip()]
    except FileNotFoundError:
        pass
    
    return commands


def get_all_child_processes(pid: int) -> List[str]:
    """Get all child process command lines recursively."""
    all_cmds = []
    
    def get_children(p: int):
        try:
            result = subprocess.run(
                ["ps", "--ppid", str(p), "-o", "pid=,comm="],
                capture_output=True,
                text=True,
                check=False
            )
            if result.returncode == 0:
                for line in result.stdout.strip().split("\n"):
                    parts = line.strip().split(None, 1)
                    if len(parts) >= 2:
                        child_pid, comm = parts
                        all_cmds.append(comm)
                        get_children(int(child_pid))
        except (ValueError, FileNotFoundError):
            pass
    
    get_children(pid)
    return all_cmds


def switch_client(target: str) -> bool:
    """Switch tmux client to target session:window.pane.
    
    Args:
        target: Target in format "session:window.pane" or pane_id
    """
    try:
        run_tmux(["switch-client", "-t", target])
        return True
    except TmuxError:
        return False


def is_tmux_running() -> bool:
    """Check if tmux server is running."""
    try:
        run_tmux(["list-sessions"], check=False)
        return True
    except TmuxError:
        return False
