#!/usr/bin/env python3
"""Idle power proxy smoke test for PAD.

This does not measure watts directly. It records idle CPU time as a
release-friendly proxy for power use. It can measure the full pad TUI,
pad-sider, or both.
"""

from __future__ import annotations

import argparse
import fcntl
import os
import pty
import shlex
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path


Result = dict[str, float | int | str]


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def default_pad_bin(root: Path) -> Path:
    return root / "rust-tui" / "target" / "debug" / "pad"


def parse_args() -> argparse.Namespace:
    root = repo_root()
    parser = argparse.ArgumentParser(
        description="Run PAD idle CPU smoke test as a power proxy."
    )
    parser.add_argument(
        "--pad-bin",
        default=os.environ.get("PAD_BIN", str(default_pad_bin(root))),
        help="path to pad binary; defaults to PAD_BIN or rust-tui/target/debug/pad",
    )
    parser.add_argument(
        "--target",
        choices=("pad-sider", "pad", "all"),
        default=os.environ.get("PAD_POWER_TARGET", "pad-sider"),
        help="component to measure",
    )
    parser.add_argument(
        "--cwd",
        default=None,
        help="workspace to open; defaults to a generated fixture",
    )
    parser.add_argument(
        "--duration",
        type=float,
        default=float(os.environ.get("PAD_POWER_DURATION", "10")),
        help="idle measurement seconds",
    )
    parser.add_argument(
        "--warmup",
        type=float,
        default=float(os.environ.get("PAD_POWER_WARMUP", "2")),
        help="startup warmup seconds before measuring full pad",
    )
    parser.add_argument(
        "--cpu-budget-pct",
        type=float,
        default=None,
        help="maximum CPU percent for all targets; overrides target budgets",
    )
    parser.add_argument(
        "--sider-cpu-budget-pct",
        type=float,
        default=None,
        help="pad-sider CPU budget; defaults to PAD_POWER_SIDER_CPU_BUDGET_PCT or 6.0",
    )
    parser.add_argument(
        "--pad-cpu-budget-pct",
        type=float,
        default=None,
        help="full pad CPU budget; defaults to PAD_POWER_PAD_CPU_BUDGET_PCT or 12.0",
    )
    parser.add_argument("--cols", type=int, default=120)
    parser.add_argument("--rows", type=int, default=40)
    parser.add_argument("--dirs", type=int, default=60)
    parser.add_argument("--files-per-dir", type=int, default=30)
    parser.add_argument(
        "--markdown-out",
        default=None,
        help="optional path for a release-ready Markdown summary",
    )
    return parser.parse_args()


def budget_for(component: str, args: argparse.Namespace) -> float:
    if args.cpu_budget_pct is not None:
        return args.cpu_budget_pct
    if component == "pad":
        if args.pad_cpu_budget_pct is not None:
            return args.pad_cpu_budget_pct
        return float(os.environ.get("PAD_POWER_PAD_CPU_BUDGET_PCT", "12"))
    if args.sider_cpu_budget_pct is not None:
        return args.sider_cpu_budget_pct
    return float(os.environ.get("PAD_POWER_SIDER_CPU_BUDGET_PCT", "6"))


def make_fixture(root: Path, dirs: int, files_per_dir: int) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "index.md").write_text("# power fixture\n", encoding="utf-8")
    for dir_index in range(dirs):
        directory = root / f"dir{dir_index:03d}"
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "index.md").write_text(
            f"# dir {dir_index}\n", encoding="utf-8"
        )
        for file_index in range(files_per_dir):
            (directory / f"file{file_index:03d}.txt").write_text(
                "idle power fixture\n" * 4,
                encoding="utf-8",
            )


def set_pty_size(fd: int, rows: int, cols: int) -> None:
    packed = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, packed)


def drain(fd: int) -> int:
    total = 0
    while True:
        try:
            chunk = os.read(fd, 65536)
        except BlockingIOError:
            return total
        except OSError:
            return total
        if not chunk:
            return total
        total += len(chunk)


def wait_with_rusage(pid: int, timeout: float) -> tuple[int, object | None]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            waited_pid, status, rusage = os.wait4(pid, os.WNOHANG)
        except ChildProcessError:
            return 0, None
        if waited_pid:
            return status, rusage
        time.sleep(0.05)

    try:
        os.kill(pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    time.sleep(0.2)
    try:
        waited_pid, status, rusage = os.wait4(pid, os.WNOHANG)
        if waited_pid:
            return status, rusage
    except ChildProcessError:
        return 0, None

    try:
        os.kill(pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    _, status, rusage = os.wait4(pid, 0)
    return status, rusage


def run_pad_sider(pad_bin: Path, cwd: Path, args: argparse.Namespace) -> Result:
    master, slave = pty.openpty()
    set_pty_size(slave, args.rows, args.cols)
    flags = fcntl.fcntl(master, fcntl.F_GETFL)
    fcntl.fcntl(master, fcntl.F_SETFL, flags | os.O_NONBLOCK)

    env = os.environ.copy()
    env.setdefault("TERM", "xterm-256color")
    command = [
        str(pad_bin),
        "__internal",
        "pad-sider",
        "ui",
        "--cwd",
        str(cwd),
    ]
    started = time.monotonic()
    proc = subprocess.Popen(
        command,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=env,
        close_fds=True,
    )
    os.close(slave)

    output_bytes = 0
    deadline = started + args.duration
    while time.monotonic() < deadline:
        output_bytes += drain(master)
        time.sleep(0.05)

    try:
        os.write(master, b"q")
    except OSError:
        pass
    status, rusage = wait_with_rusage(proc.pid, 2.0)
    output_bytes += drain(master)
    os.close(master)

    elapsed = time.monotonic() - started
    cpu_seconds = 0.0
    maxrss = 0
    if rusage is not None:
        cpu_seconds = float(rusage.ru_utime + rusage.ru_stime)
        maxrss = int(rusage.ru_maxrss)

    return result_for("pad-sider", status, elapsed, cpu_seconds, output_bytes, maxrss)


def result_for(
    component: str,
    status: int,
    elapsed: float,
    cpu_seconds: float,
    output_bytes: int,
    maxrss: int,
) -> Result:
    return {
        "component": component,
        "status": int(status),
        "elapsed": elapsed,
        "cpu_seconds": cpu_seconds,
        "cpu_pct": cpu_seconds / elapsed * 100.0 if elapsed > 0 else 0.0,
        "output_bytes": output_bytes,
        "maxrss_raw": maxrss,
    }


def proc_cpu_seconds(pid: int) -> float:
    stat_path = Path(f"/proc/{pid}/stat")
    if not stat_path.exists():
        raise RuntimeError("full pad power smoke currently requires /proc")
    text = stat_path.read_text(encoding="utf-8")
    fields = text.rsplit(")", 1)[1].split()
    ticks = int(fields[11]) + int(fields[12])
    return ticks / os.sysconf(os.sysconf_names["SC_CLK_TCK"])


def proc_maxrss_kb(pid: int) -> int:
    status_path = Path(f"/proc/{pid}/status")
    if not status_path.exists():
        return 0
    for line in status_path.read_text(encoding="utf-8").splitlines():
        if line.startswith("VmHWM:"):
            parts = line.split()
            if len(parts) >= 2:
                return int(parts[1])
    return 0


def tmux(sock: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["tmux", "-S", str(sock), *args],
        check=check,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def wait_for_pad_pid(sock: Path, timeout: float) -> int:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        result = tmux(
            sock,
            "display-message",
            "-p",
            "-t",
            "pad:0.0",
            "#{pane_pid}",
            check=False,
        )
        if result.returncode == 0 and result.stdout.strip().isdigit():
            pid = int(result.stdout.strip())
            if Path(f"/proc/{pid}/stat").exists():
                return pid
        time.sleep(0.1)
    raise RuntimeError("timed out waiting for pad pane pid")


def shell_cmd(command: str) -> str:
    return f"/bin/sh -lc {shlex.quote(command)}"


def run_full_pad(pad_bin: Path, cwd: Path, args: argparse.Namespace) -> Result:
    if not shutil.which("tmux"):
        raise RuntimeError("full pad power smoke requires tmux")
    tmp = Path(tempfile.mkdtemp(prefix="pad-power-tmux-"))
    sock = tmp / "tmux.sock"
    home = tmp / "home"
    home.mkdir(parents=True, exist_ok=True)
    root = repo_root()
    mock_agent = root / "scripts" / "ci" / "mock_agent.sh"
    output_bytes = 0
    try:
        tmux(
            sock,
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            "agents",
            "-n",
            "agents",
            "-x",
            str(args.cols),
            "-y",
            str(args.rows),
            shell_cmd(f"cd {shlex.quote(str(cwd))} && {shlex.quote(str(mock_agent))} codex"),
        )
        tmux(
            sock,
            "split-window",
            "-t",
            "agents:0",
            "-h",
            shell_cmd(f"cd {shlex.quote(str(cwd))} && {shlex.quote(str(mock_agent))} claude"),
        )
        tmux(
            sock,
            "new-session",
            "-d",
            "-s",
            "pad",
            "-x",
            str(args.cols),
            "-y",
            str(args.rows),
            shell_cmd(
                " ".join(
                    [
                        f"export HOME={shlex.quote(str(home))};",
                        "export TERM=xterm-256color;",
                        f"cd {shlex.quote(str(cwd))};",
                        f"exec {shlex.quote(str(pad_bin))} --debug",
                    ]
                )
            ),
        )
        pid = wait_for_pad_pid(sock, args.warmup + 5.0)
        time.sleep(args.warmup)
        start_cpu = proc_cpu_seconds(pid)
        started = time.monotonic()
        deadline = started + args.duration
        while time.monotonic() < deadline:
            time.sleep(0.25)
        elapsed = time.monotonic() - started
        end_cpu = proc_cpu_seconds(pid)
        maxrss = proc_maxrss_kb(pid)
        capture = tmux(sock, "capture-pane", "-p", "-t", "pad:0.0", check=False)
        output_bytes = len(capture.stdout.encode("utf-8", errors="replace"))
        tmux(sock, "send-keys", "-t", "pad:0.0", "q", check=False)
        time.sleep(0.5)
        alive = Path(f"/proc/{pid}/stat").exists()
        if alive:
            tmux(sock, "kill-server", check=False)
        status = 0
        cpu_seconds = max(0.0, end_cpu - start_cpu)
        return result_for("pad", status, elapsed, cpu_seconds, output_bytes, maxrss)
    finally:
        tmux(sock, "kill-server", check=False)
        shutil.rmtree(tmp, ignore_errors=True)


def write_markdown_summary(path: Path, results: list[Result], args: argparse.Namespace) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "## Power smoke",
        "",
        "PAD does not measure watts directly. This release records idle CPU as a power proxy.",
        "",
        "| Component | Elapsed | CPU time | Average CPU | Budget | Terminal output |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for result in results:
        component = str(result["component"])
        budget = budget_for(component, args)
        lines.append(
            f"| {component} | {result['elapsed']:.2f}s | "
            f"{result['cpu_seconds']:.4f}s | {result['cpu_pct']:.2f}% | "
            f"{budget:.2f}% | {result['output_bytes']} bytes |"
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def print_result(result: Result, args: argparse.Namespace) -> None:
    component = str(result["component"])
    budget = budget_for(component, args)
    print(
        f"{component} power smoke: "
        f"elapsed={result['elapsed']:.2f}s "
        f"cpu={result['cpu_seconds']:.4f}s "
        f"cpu_pct={result['cpu_pct']:.2f}% "
        f"budget={budget:.2f}% "
        f"output_bytes={result['output_bytes']} "
        f"maxrss_raw={result['maxrss_raw']}"
    )


def main() -> int:
    args = parse_args()
    pad_bin = Path(args.pad_bin).expanduser().resolve()
    if not pad_bin.is_file() or not os.access(pad_bin, os.X_OK):
        print(
            f"pad binary is not executable: {pad_bin}\n"
            "Build it first, for example: cargo build --manifest-path rust-tui/Cargo.toml",
            file=sys.stderr,
        )
        return 2

    temp_dir = None
    if args.cwd:
        cwd = Path(args.cwd).expanduser().resolve()
    else:
        temp_dir = Path(tempfile.mkdtemp(prefix="pad-power-"))
        cwd = temp_dir / "workspace"
        make_fixture(cwd, args.dirs, args.files_per_dir)

    try:
        results: list[Result] = []
        if args.target in ("pad-sider", "all"):
            results.append(run_pad_sider(pad_bin, cwd, args))
        if args.target in ("pad", "all"):
            results.append(run_full_pad(pad_bin, cwd, args))
    finally:
        if temp_dir is not None:
            shutil.rmtree(temp_dir, ignore_errors=True)

    failed = False
    for result in results:
        print_result(result, args)
        component = str(result["component"])
        if result["cpu_pct"] > budget_for(component, args):
            print(
                f"{component} CPU budget exceeded. This is a power proxy failure; "
                "check idle redraws, refresh cadence, and file-system scans.",
                file=sys.stderr,
            )
            failed = True
        if result["status"] != 0:
            print(
                f"{component} exited with status {result['status']}",
                file=sys.stderr,
            )
            failed = True

    if args.markdown_out:
        write_markdown_summary(Path(args.markdown_out), results, args)
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
