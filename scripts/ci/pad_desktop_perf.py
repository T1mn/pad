#!/usr/bin/env python3
"""Performance gate for the final packaged PAD Desktop Electron app."""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import re
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_APP = ROOT / "apps/pad-desktop/out/PAD Desktop-darwin-arm64/PAD Desktop.app"
COLD_START_LIMIT_SECONDS = 3.0
IDLE_CPU_LIMIT_PERCENT = 2.0
RSS_LIMIT_MIB = 450.0
IDLE_SECONDS = 10.0
EXPECTED_PROTOCOL_MINIMUM = 2
PROCESS_REPORT_NAME = "pad-desktop-perf-process-family.json"
PROTECTED_INSTALLED_APP = pathlib.Path("/Applications/PAD Desktop.app")


class PerfError(RuntimeError):
    pass


@dataclass(frozen=True)
class ProcessMetric:
    pid: int
    ppid: int
    pgid: int
    cpu_seconds: float
    rss_kib: int
    started_at: str
    command: str


def require(condition: bool, message: str) -> None:
    if not condition:
        raise PerfError(message)


def inherited_environment_allowlist() -> dict[str, str]:
    exact = {"HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "PATH"}
    return {
        key: value
        for key, value in os.environ.items()
        if key in exact or key.startswith("LC_")
    }


def isolated_environment(home: pathlib.Path, data_root: pathlib.Path) -> dict[str, str]:
    env = inherited_environment_allowlist()
    env.update(
        {
            "HOME": str(home),
            "PAD_DESKTOP_DATA_DIR": str(data_root),
            "XDG_CACHE_HOME": str(home / ".cache"),
            "XDG_CONFIG_HOME": str(home / ".config"),
        }
    )
    return env


def run_command(
    args: list[str],
    *,
    env: dict[str, str] | None = None,
    timeout: float = 30.0,
) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            args,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=env,
            timeout=timeout,
        )
    except subprocess.CalledProcessError as error:
        detail = (error.stderr or error.stdout or "").strip()
        raise PerfError(f"command failed ({' '.join(args)}): {detail}") from error
    except subprocess.TimeoutExpired as error:
        raise PerfError(f"command timed out: {' '.join(args)}") from error


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def wait_for_target(port: int, process: subprocess.Popen[bytes], timeout: float = 15.0) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise PerfError(f"PAD Desktop exited during cold start: {process.returncode}")
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/list", timeout=0.5) as response:
                targets = json.load(response)
            for target in targets:
                if target.get("type") == "page" and target.get("webSocketDebuggerUrl"):
                    return target
        except (OSError, urllib.error.URLError, json.JSONDecodeError):
            pass
        time.sleep(0.025)
    raise PerfError("CDP target did not appear within 15 seconds")


CDP_READY_SCRIPT = r"""
const url = process.env.PAD_CDP_WS;
if (!url) throw new Error("missing PAD_CDP_WS");
const socket = new WebSocket(url);
const pending = new Map();
let sequence = 0;
function call(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error(`timeout: ${method}`)); }, 10000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
socket.addEventListener("message", (event) => {
  const message = JSON.parse(String(event.data));
  if (!message.id) return;
  const item = pending.get(message.id);
  if (!item) return;
  clearTimeout(item.timer);
  pending.delete(message.id);
  if (message.error) item.reject(new Error(JSON.stringify(message.error)));
  else item.resolve(message.result);
});
socket.addEventListener("open", async () => {
  try {
    await call("Page.enable");
    await call("Runtime.enable");
    await call("Page.bringToFront");
    const response = await call("Runtime.evaluate", {
      expression: `(async () => {
        const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
        let domEpochMs = null;
        let rendererReadyEpochMs = null;
        let readySamples = 0;
        for (let attempt = 0; attempt < 600; attempt += 1) {
          const shell = document.querySelector(".app-shell");
          if (domEpochMs === null && document.readyState === "complete" && shell) {
            domEpochMs = performance.timeOrigin + performance.now();
          }
          const ready = domEpochMs !== null
            && shell
            && !shell.classList.contains("is-loading")
            && !document.querySelector(".task-data-loading");
          readySamples = ready ? readySamples + 1 : 0;
          if (readySamples >= 5) {
            rendererReadyEpochMs = performance.timeOrigin + performance.now();
            break;
          }
          await delay(10);
        }
        if (domEpochMs === null) throw new Error("DOM shell did not become ready");
        if (rendererReadyEpochMs === null) throw new Error("renderer shell remained loading");
        const visible = (element) => {
          const rect = element.getBoundingClientRect();
          const style = getComputedStyle(element);
          return rect.width > 0 && rect.height > 0
            && style.display !== "none" && style.visibility !== "hidden";
        };
        const alerts = [...document.querySelectorAll('[role="alert"], .error-banner')]
          .filter(visible)
          .map((element) => element.textContent?.trim() || "")
          .filter(Boolean);
        if (typeof window.padDesktop?.bootstrap !== "function") throw new Error("preload API is unavailable");
        const bootstrapStarted = performance.now();
        const bootstrap = await window.padDesktop.bootstrap();
        const bootstrapMs = performance.now() - bootstrapStarted;
        return {
          domEpochMs,
          rendererReadyEpochMs,
          bootstrapCompletedEpochMs: performance.timeOrigin + performance.now(),
          bootstrapMs,
          protocolVersion: bootstrap?.protocol_version ?? null,
          backendStatus: bootstrap?.backend?.status ?? null,
          alerts,
        };
      })()`,
      awaitPromise: true,
      returnByValue: true,
    });
    if (response.exceptionDetails) throw new Error(JSON.stringify(response.exceptionDetails));
    process.stdout.write(JSON.stringify(response.result.value));
    socket.close();
  } catch (error) {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exitCode = 1;
    socket.close();
  }
});
setTimeout(() => process.exit(2), 30000).unref();
"""


CDP_HEAP_SCRIPT = r"""
const url = process.env.PAD_CDP_WS;
if (!url) throw new Error("missing PAD_CDP_WS");
const socket = new WebSocket(url);
let sequence = 0;
const pending = new Map();
function call(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
socket.addEventListener("message", (event) => {
  const message = JSON.parse(String(event.data));
  const item = pending.get(message.id);
  if (!item) return;
  pending.delete(message.id);
  if (message.error) item.reject(new Error(JSON.stringify(message.error)));
  else item.resolve(message.result);
});
socket.addEventListener("open", async () => {
  try {
    const usage = await call("Runtime.getHeapUsage");
    process.stdout.write(JSON.stringify(usage));
    socket.close();
  } catch (error) {
    console.error(String(error));
    process.exitCode = 1;
    socket.close();
  }
});
setTimeout(() => process.exit(2), 10000).unref();
"""


CDP_CLOSE_SCRIPT = r"""
const url = process.env.PAD_CDP_WS;
const socket = new WebSocket(url);
socket.addEventListener("open", () => socket.send(JSON.stringify({ id: 1, method: "Browser.close" })));
socket.addEventListener("message", () => process.exit(0));
setTimeout(() => process.exit(0), 1000);
"""


def cdp_json(bun: pathlib.Path, websocket_url: str, script: str, timeout: float) -> dict[str, Any]:
    env = inherited_environment_allowlist()
    env["PAD_CDP_WS"] = websocket_url
    result = run_command([str(bun), "-e", script], env=env, timeout=timeout)
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise PerfError(f"CDP helper returned invalid JSON: {result.stdout[:300]}") from error
    require(isinstance(value, dict), "CDP helper result is not an object")
    return value


def cpu_time_seconds(value: str) -> float:
    days = 0
    clock = value
    if "-" in value:
        day_text, clock = value.split("-", 1)
        days = int(day_text)
    parts = clock.split(":")
    require(len(parts) in (2, 3), f"unsupported ps CPU time: {value}")
    if len(parts) == 2:
        hours = 0
        minutes, seconds = parts
    else:
        hours, minutes, seconds = parts
    return days * 86400 + int(hours) * 3600 + int(minutes) * 60 + float(seconds)


def process_metrics() -> dict[int, ProcessMetric]:
    output = run_command(
        ["/bin/ps", "-axo", "pid=,ppid=,pgid=,time=,rss=,lstart=,command="]
    ).stdout
    metrics: dict[int, ProcessMetric] = {}
    for line in output.splitlines():
        parts = line.strip().split(None, 10)
        if (
            len(parts) != 11
            or not parts[0].isdigit()
            or not parts[1].isdigit()
            or not parts[2].lstrip("-").isdigit()
        ):
            continue
        pid, ppid, pgid, cpu, rss = parts[:5]
        if not rss.isdigit():
            continue
        metrics[int(pid)] = ProcessMetric(
            pid=int(pid),
            ppid=int(ppid),
            pgid=int(pgid),
            cpu_seconds=cpu_time_seconds(cpu),
            rss_kib=int(rss),
            started_at=" ".join(parts[5:10]),
            command=parts[10],
        )
    return metrics


def metric_json(metric: ProcessMetric, app: pathlib.Path, root_pid: int) -> dict[str, Any]:
    resources = app / "Contents/Resources"
    if metric.pid == root_pid:
        role = "electron_main"
    elif str(resources / "pad") in metric.command:
        role = "rust_control_plane"
    elif str(app / "Contents/Frameworks") in metric.command:
        role = "electron_helper"
    elif str(app) in metric.command:
        role = "packaged_app_descendant"
    else:
        role = "pty_or_app_descendant"
    return {
        "pid": metric.pid,
        "ppid": metric.ppid,
        "pgid": metric.pgid,
        "startedAt": metric.started_at,
        "role": role,
        "cpuSeconds": metric.cpu_seconds,
        "rssKiB": metric.rss_kib,
        "command": metric.command,
    }


def bundle_processes(app: pathlib.Path) -> list[ProcessMetric]:
    prefix = f"{app}/"
    return sorted(
        (metric for metric in process_metrics().values() if metric.command.startswith(prefix)),
        key=lambda metric: metric.pid,
    )


class ProcessFamilyTracker:
    def __init__(self, process: subprocess.Popen[bytes], app: pathlib.Path) -> None:
        self.process = process
        self.app = app
        self.root_pid = process.pid
        try:
            self.pgid = os.getpgid(process.pid)
        except ProcessLookupError as error:
            raise PerfError("PAD Desktop exited before its process group was recorded") from error
        require(
            self.pgid == self.root_pid,
            f"isolated process group mismatch: pid={self.root_pid}, pgid={self.pgid}",
        )
        self.known: dict[tuple[int, str], ProcessMetric] = {}
        self.snapshots: list[dict[str, Any]] = []

    @staticmethod
    def _identity(metric: ProcessMetric) -> tuple[int, str]:
        return (metric.pid, metric.started_at)

    def _is_known(self, metric: ProcessMetric) -> bool:
        return self._identity(metric) in self.known

    def current(self) -> dict[int, ProcessMetric]:
        table = process_metrics()
        family = {metric.pid for metric in table.values() if metric.pgid == self.pgid}
        family.update(
            metric.pid for metric in table.values() if self._identity(metric) in self.known
        )
        if self.root_pid in table:
            family.add(self.root_pid)
        changed = True
        while changed:
            changed = False
            for metric in table.values():
                if metric.ppid in family and metric.pid not in family:
                    family.add(metric.pid)
                    changed = True
        sample = {pid: table[pid] for pid in family if pid in table}
        for metric in sample.values():
            self.known[self._identity(metric)] = metric
        return sample

    def capture(self, label: str) -> dict[int, ProcessMetric]:
        sample = self.current()
        self.snapshots.append(
            {
                "label": label,
                "capturedAtEpochMs": time.time_ns() // 1_000_000,
                "leaderExited": self.process.poll() is not None,
                "processes": [
                    metric_json(metric, self.app, self.root_pid)
                    for metric in sorted(sample.values(), key=lambda item: item.pid)
                ],
            }
        )
        return sample

    def _assert_safe(self, sample: dict[int, ProcessMetric]) -> None:
        protected_prefix = f"{PROTECTED_INSTALLED_APP}/"
        protected = [
            metric.pid for metric in sample.values() if protected_prefix in metric.command
        ]
        require(
            not protected,
            "refusing cleanup because /Applications/PAD Desktop.app appeared in "
            f"the test process family: {protected}",
        )
        group = [metric for metric in sample.values() if metric.pgid == self.pgid]
        if group:
            require(
                any(self._is_known(metric) or str(self.app) in metric.command for metric in group),
                "refusing to signal an unrecognized process group",
            )

    def _signal(self, sample: dict[int, ProcessMetric], sig: signal.Signals) -> None:
        self._assert_safe(sample)
        if any(metric.pgid == self.pgid for metric in sample.values()):
            try:
                os.killpg(self.pgid, sig)
            except ProcessLookupError:
                pass
        for metric in sample.values():
            if metric.pgid == self.pgid or not self._is_known(metric):
                continue
            try:
                os.kill(metric.pid, sig)
            except ProcessLookupError:
                pass

    def _wait_empty(self, timeout: float) -> dict[int, ProcessMetric]:
        deadline = time.monotonic() + timeout
        sample = self.current()
        while sample and time.monotonic() < deadline:
            time.sleep(0.05)
            sample = self.current()
        return sample

    def cleanup(self) -> dict[str, Any]:
        sample = self.capture("before_cleanup")
        if sample:
            self._signal(sample, signal.SIGTERM)
            sample = self._wait_empty(4)
        if sample:
            self._signal(sample, signal.SIGKILL)
            sample = self._wait_empty(2)
        try:
            self.process.wait(timeout=0.2)
        except subprocess.TimeoutExpired:
            pass
        residuals = self.capture("after_cleanup")
        evidence = self.evidence(residuals)
        require(
            not residuals,
            "PAD Desktop process family survived cleanup: "
            + ", ".join(f"{pid}:{metric.command}" for pid, metric in residuals.items()),
        )
        return evidence

    def evidence(self, residuals: dict[int, ProcessMetric] | None = None) -> dict[str, Any]:
        current = residuals if residuals is not None else self.current()
        history = sorted(self.known.values(), key=lambda item: (item.started_at, item.pid))
        return {
            "rootPid": self.root_pid,
            "processGroupId": self.pgid,
            "testedBundle": str(self.app),
            "protectedInstalledBundle": str(PROTECTED_INSTALLED_APP),
            "snapshots": self.snapshots,
            "observedProcesses": [metric_json(item, self.app, self.root_pid) for item in history],
            "residualProcesses": [
                metric_json(item, self.app, self.root_pid)
                for item in sorted(current.values(), key=lambda value: value.pid)
            ],
            "cleanupPassed": not current,
        }


def idle_metrics(
    process: subprocess.Popen[bytes], tracker: ProcessFamilyTracker
) -> dict[str, Any]:
    before = tracker.capture("idle_started")
    require(process.pid in before, "main process disappeared before idle sampling")
    base_cpu = {pid: metric.cpu_seconds for pid, metric in before.items()}
    final_cpu = dict(base_cpu)
    rss_samples: list[float] = []
    process_counts: list[int] = []
    started = time.monotonic()
    deadline = started + IDLE_SECONDS
    while True:
        if process.poll() is not None:
            raise PerfError(f"PAD Desktop exited during idle sampling: {process.returncode}")
        sample = tracker.current()
        require(process.pid in sample, "main process vanished during idle sampling")
        for pid, metric in sample.items():
            final_cpu[pid] = max(final_cpu.get(pid, 0.0), metric.cpu_seconds)
        rss_samples.append(sum(metric.rss_kib for metric in sample.values()) / 1024.0)
        process_counts.append(len(sample))
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            break
        time.sleep(min(1.0, remaining))
    elapsed = time.monotonic() - started
    cpu_delta = sum(
        max(0.0, final - base_cpu.get(pid, 0.0)) for pid, final in final_cpu.items()
    )
    return {
        "window_seconds": elapsed,
        "cpu_seconds": cpu_delta,
        "cpu_percent": (cpu_delta / elapsed) * 100.0,
        "rss_peak_mib": max(rss_samples),
        "rss_end_mib": rss_samples[-1],
        "process_count_peak": max(process_counts),
    }


def run_perf(app: pathlib.Path, artifact_dir: pathlib.Path) -> dict[str, Any]:
    app = app.expanduser().resolve()
    require(app == DEFAULT_APP.resolve(), f"final app must be tested at {DEFAULT_APP}")
    executable = app / "Contents/MacOS/PADDesktop"
    bun = app / "Contents/Resources/bin/bun"
    require(executable.is_file() and os.access(executable, os.X_OK), f"missing {executable}")
    require(bun.is_file() and os.access(bun, os.X_OK), f"missing {bun}")
    existing = [metric.pid for metric in bundle_processes(app)]
    require(not existing, f"another packaged PAD Desktop is running: {existing}")

    scratch = pathlib.Path(tempfile.mkdtemp(prefix="pad-desktop-perf-state-"))
    home = scratch / "home"
    data = scratch / "data"
    user_data = scratch / "electron-user-data"
    for directory in (home, data, user_data):
        directory.mkdir(mode=0o700)
    port = free_port()
    env = isolated_environment(home, data)
    stdout_path = artifact_dir / "perf-app-stdout.log"
    stderr_path = artifact_dir / "perf-app-stderr.log"
    app_started_epoch_ms = time.time_ns() / 1_000_000
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(
            [
                str(executable),
                f"--remote-debugging-port={port}",
                "--remote-allow-origins=*",
                f"--user-data-dir={user_data}",
                "--no-first-run",
                "--disable-default-apps",
            ],
            env=env,
            stdout=stdout,
            stderr=stderr,
            start_new_session=True,
        )

    tracker = ProcessFamilyTracker(process, app)
    tracker.capture("launched")
    result: dict[str, Any] | None = None
    process_family: dict[str, Any] | None = None
    cleanup_error: Exception | None = None
    try:
        target = wait_for_target(port, process)
        tracker.capture("cdp_ready")
        websocket_url = str(target["webSocketDebuggerUrl"])
        ready = cdp_json(bun, websocket_url, CDP_READY_SCRIPT, 35)
        cold_dom_seconds = (float(ready["domEpochMs"]) - app_started_epoch_ms) / 1000.0
        cold_renderer_ready_seconds = (
            float(ready["rendererReadyEpochMs"]) - app_started_epoch_ms
        ) / 1000.0
        cold_bootstrap_seconds = (
            float(ready["bootstrapCompletedEpochMs"]) - app_started_epoch_ms
        ) / 1000.0
        require(cold_dom_seconds >= 0, "cold DOM clock measurement is invalid")
        require(
            cold_renderer_ready_seconds >= cold_dom_seconds,
            "renderer readiness preceded DOM readiness",
        )
        require(
            cold_bootstrap_seconds >= cold_renderer_ready_seconds,
            "validation bootstrap preceded renderer readiness",
        )

        idle = idle_metrics(process, tracker)
        tracker.capture("idle_completed")
        heap = cdp_json(bun, websocket_url, CDP_HEAP_SCRIPT, 15)
        helper_env = inherited_environment_allowlist()
        helper_env["PAD_CDP_WS"] = websocket_url
        try:
            run_command([str(bun), "-e", CDP_CLOSE_SCRIPT], env=helper_env, timeout=3)
            process.wait(timeout=6)
        except (PerfError, subprocess.TimeoutExpired):
            pass

        measurements = {
            "cold_dom_seconds": cold_dom_seconds,
            "cold_renderer_ready_seconds": cold_renderer_ready_seconds,
            "cold_bootstrap_seconds": cold_bootstrap_seconds,
            "bootstrap_call_seconds": float(ready["bootstrapMs"]) / 1000.0,
            **idle,
            "renderer_js_heap_used_mib": float(heap["usedSize"]) / (1024 * 1024),
            "renderer_js_heap_total_mib": float(heap["totalSize"]) / (1024 * 1024),
            "protocol_version": ready.get("protocolVersion"),
            "backend_status": ready.get("backendStatus"),
            "renderer_alerts": ready.get("alerts", []),
        }
        failures: list[str] = []
        if cold_renderer_ready_seconds > COLD_START_LIMIT_SECONDS:
            failures.append(
                "cold renderer readiness "
                f"{cold_renderer_ready_seconds:.3f}s > {COLD_START_LIMIT_SECONDS:.1f}s"
            )
        if float(idle["cpu_percent"]) > IDLE_CPU_LIMIT_PERCENT:
            failures.append(
                f"idle CPU {idle['cpu_percent']:.3f}% > {IDLE_CPU_LIMIT_PERCENT:.1f}%"
            )
        if float(idle["rss_peak_mib"]) > RSS_LIMIT_MIB:
            failures.append(f"RSS {idle['rss_peak_mib']:.1f}MiB > {RSS_LIMIT_MIB:.0f}MiB")
        protocol_version = ready.get("protocolVersion")
        if (
            not isinstance(protocol_version, int)
            or isinstance(protocol_version, bool)
            or protocol_version < EXPECTED_PROTOCOL_MINIMUM
        ):
            failures.append(
                f"protocol {protocol_version!r} is older than v{EXPECTED_PROTOCOL_MINIMUM}"
            )
        if ready.get("backendStatus") != "ready":
            failures.append(f"backend status is {ready.get('backendStatus')!r}, expected 'ready'")
        if ready.get("alerts"):
            failures.append(f"renderer exposed alerts: {ready.get('alerts')!r}")
        result = {
            "app": str(app),
            "thresholds": {
                "cold_start_seconds": COLD_START_LIMIT_SECONDS,
                "idle_cpu_percent": IDLE_CPU_LIMIT_PERCENT,
                "rss_mib": RSS_LIMIT_MIB,
                "idle_window_seconds": IDLE_SECONDS,
                "protocol_minimum": EXPECTED_PROTOCOL_MINIMUM,
            },
            "measurements": measurements,
            "pass": not failures,
            "failures": failures,
        }
    finally:
        try:
            process_family = tracker.cleanup()
        except (PerfError, OSError) as error:
            cleanup_error = error
            process_family = tracker.evidence()
        (artifact_dir / PROCESS_REPORT_NAME).write_text(
            json.dumps(process_family, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        if result is not None:
            result["process_family"] = process_family
        shutil.rmtree(scratch, ignore_errors=True)
    if cleanup_error is not None:
        raise PerfError(f"process cleanup failed: {cleanup_error}") from cleanup_error
    require(result is not None, "performance result was not produced")
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=pathlib.Path, default=DEFAULT_APP)
    parser.add_argument("--artifact-dir", type=pathlib.Path, required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    artifact_dir = args.artifact_dir.expanduser().resolve()
    artifact_dir.mkdir(parents=True, exist_ok=True)
    result_path = artifact_dir / "pad-desktop-perf.json"
    try:
        result = run_perf(args.app, artifact_dir)
    except Exception as error:
        result = {
            "app": str(args.app.expanduser()),
            "thresholds": {
                "cold_start_seconds": COLD_START_LIMIT_SECONDS,
                "idle_cpu_percent": IDLE_CPU_LIMIT_PERCENT,
                "rss_mib": RSS_LIMIT_MIB,
                "idle_window_seconds": IDLE_SECONDS,
                "protocol_minimum": EXPECTED_PROTOCOL_MINIMUM,
            },
            "pass": False,
            "failures": [str(error)],
        }
    result_path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if result.get("pass") is True:
        measurements = result["measurements"]
        print(
            "[PASS] "
            f"cold={measurements['cold_renderer_ready_seconds']:.3f}s "
            f"bootstrap_check={measurements['bootstrap_call_seconds']:.3f}s "
            f"idle_cpu={measurements['cpu_percent']:.3f}% "
            f"rss={measurements['rss_peak_mib']:.1f}MiB "
            f"heap={measurements['renderer_js_heap_used_mib']:.1f}MiB"
        )
        print(f"[PASS] result: {result_path}")
        return 0
    print(f"[FAIL] {'; '.join(result.get('failures', []))}", file=sys.stderr)
    print(f"[INFO] result: {result_path}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
