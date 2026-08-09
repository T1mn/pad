#!/usr/bin/env python3
"""Exercise the packaged PAD binary through its native PTY terminal."""

from __future__ import annotations

import fcntl
import os
import pty
import shlex
import struct
import subprocess
import sys
import tempfile
import termios
import time
from pathlib import Path


def set_pty_size(fd: int, rows: int = 40, cols: int = 120) -> None:
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))


def drain(fd: int, output: bytearray) -> None:
    while True:
        try:
            chunk = os.read(fd, 65536)
        except (BlockingIOError, OSError):
            return
        if not chunk:
            return
        output.extend(chunk)


def wait_until(proc: subprocess.Popen[bytes], fd: int, output: bytearray, deadline: float, predicate) -> bool:
    while time.monotonic() < deadline:
        drain(fd, output)
        if predicate():
            return True
        if proc.poll() is not None:
            return predicate()
        time.sleep(0.05)
    drain(fd, output)
    return predicate()


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {Path(sys.argv[0]).name} PAD_BIN", file=sys.stderr)
        return 2

    pad_bin = Path(sys.argv[1]).expanduser().resolve()
    if not pad_bin.is_file() or not os.access(pad_bin, os.X_OK):
        print(f"pad binary is not executable: {pad_bin}", file=sys.stderr)
        return 2

    with tempfile.TemporaryDirectory(prefix="pad-native-smoke-") as temp:
        root = Path(temp)
        workspace = root / "workspace"
        workspace.mkdir()
        (workspace / "index.md").write_text("# native smoke\n", encoding="utf-8")
        marker = root / "shell-input-ok"

        master, slave = pty.openpty()
        set_pty_size(slave)
        os.set_blocking(master, False)
        env = os.environ.copy()
        env.update(
            {
                "HOME": str(root / "home"),
                "PAD_HOME": str(root / "pad-home"),
                "TERM": "xterm-256color",
            }
        )
        Path(env["HOME"]).mkdir()
        output = bytearray()
        proc = subprocess.Popen(
            [str(pad_bin), "--debug"],
            cwd=workspace,
            stdin=slave,
            stdout=slave,
            stderr=slave,
            env=env,
            close_fds=True,
        )
        os.close(slave)

        try:
            ready = wait_until(
                proc,
                master,
                output,
                time.monotonic() + 8.0,
                lambda: len(output) > 0,
            )
            if not ready:
                raise RuntimeError("PAD did not render its initial frame")

            command_layer_start = len(output)
            os.write(master, b"\x07")  # Ctrl+G: portable PAD Terminal prefix.
            if not wait_until(
                proc,
                master,
                output,
                time.monotonic() + 3.0,
                lambda: b"PAD TERM" in output[command_layer_start:],
            ):
                raise RuntimeError("Ctrl+G did not open the PAD Terminal command layer")
            os.write(master, b"\x07")
            time.sleep(0.2)
            os.write(master, b"\x1b[24~")
            time.sleep(0.2)

            os.write(master, b"\t")
            time.sleep(0.4)
            command = f"printf native-pty-ok > {shlex.quote(str(marker))}\r"
            os.write(master, command.encode())
            if not wait_until(
                proc,
                master,
                output,
                time.monotonic() + 8.0,
                marker.is_file,
            ):
                raise RuntimeError("input did not reach the PAD-owned shell PTY")

            os.write(master, b"\x1b[24~")
            time.sleep(0.3)
            os.write(master, b"q")
            if not wait_until(
                proc,
                master,
                output,
                time.monotonic() + 10.0,
                lambda: proc.poll() is not None,
            ):
                raise RuntimeError("PAD did not exit after returning from the terminal")
            if proc.returncode != 0:
                raise RuntimeError(f"PAD exited with status {proc.returncode}")
        except Exception as error:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait()
            drain(master, output)
            print(output.decode("utf-8", errors="replace")[-4000:], file=sys.stderr)
            print(f"native terminal smoke failed: {error}", file=sys.stderr)
            return 1
        finally:
            os.close(master)

    print("native terminal smoke passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
