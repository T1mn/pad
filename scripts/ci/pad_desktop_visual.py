#!/usr/bin/env python3
"""Capture and validate the PAD Desktop macOS visual matrix.

The packaged Electron app is launched with an isolated HOME, Electron profile,
and PAD_DESKTOP_DATA_DIR.  Chrome DevTools Protocol (CDP) then captures the
light/dark matrix at 1280x820, 1440x900, 960x720, 720x700, and 480x600 while
recording layout, localization, focus-visible, and basic ARIA evidence.

An optional baseline directory may contain PNG files with the same names as the
generated captures (for example ``light-1280x820.png``).  Only a supplied,
dimension-matched image is compared.  Without one, similarity is reported as
``not_evaluated``; this script never invents a Codex SSIM score.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import os
import pathlib
import platform
import shutil
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import zlib
from dataclasses import dataclass
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_APP = ROOT / "apps/pad-desktop/out/PAD Desktop-darwin-arm64/PAD Desktop.app"
REPORT_NAME = "pad-desktop-visual-report.json"
PROCESS_REPORT_NAME = "pad-desktop-visual-process-family.json"
PROTECTED_INSTALLED_APP = pathlib.Path("/Applications/PAD Desktop.app")
MATRIX = (
    ("light", 1280, 820),
    ("light", 1440, 900),
    ("light", 960, 720),
    ("light", 720, 700),
    ("light", 480, 600),
    ("dark", 1280, 820),
    ("dark", 1440, 900),
    ("dark", 960, 720),
    ("dark", 720, 700),
    ("dark", 480, 600),
)


class VisualError(RuntimeError):
    """A clear, user-facing visual acceptance failure."""


@dataclass(frozen=True)
class Bundle:
    app: pathlib.Path
    executable: pathlib.Path
    bun: pathlib.Path


@dataclass(frozen=True)
class ProcessRecord:
    pid: int
    ppid: int
    pgid: int
    started_at: str
    command: str


@dataclass(frozen=True)
class PngImage:
    width: int
    height: int
    color_type: int
    bytes_per_pixel: int
    pixels: bytes


def require(condition: bool, message: str) -> None:
    if not condition:
        raise VisualError(message)


def inherited_environment_allowlist() -> dict[str, str]:
    exact_keys = {"HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "PATH"}
    return {
        key: value
        for key, value in os.environ.items()
        if key in exact_keys or key.startswith("LC_")
    }


def safe_environment(home: pathlib.Path, data_root: pathlib.Path) -> dict[str, str]:
    env = inherited_environment_allowlist()
    env.update(
        {
            "HOME": str(home),
            "PAD_DESKTOP_DATA_DIR": str(data_root),
            "XDG_CACHE_HOME": str(home / ".cache"),
            "XDG_CONFIG_HOME": str(home / ".config"),
            "ELECTRON_ENABLE_LOGGING": "1",
        }
    )
    return env


def free_tcp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def check_bundle(app_argument: pathlib.Path) -> Bundle:
    require(platform.system() == "Darwin", "PAD Desktop visual acceptance is macOS-only")
    app = app_argument.expanduser().resolve()
    executable = app / "Contents/MacOS/PADDesktop"
    resources = app / "Contents/Resources"
    required = (
        app / "Contents/Info.plist",
        executable,
        resources / "pad",
        resources / "bin/bun",
        resources / "bin/pi",
        resources / "pi/package.json",
    )
    missing = [str(path) for path in required if not path.is_file()]
    require(
        not missing,
        "incomplete PAD Desktop bundle; missing "
        + ", ".join(missing)
        + ". Build the full packaged app first, or run --syntax-only.",
    )
    for binary in (executable, resources / "pad", resources / "bin/bun", resources / "bin/pi"):
        require(os.access(binary, os.X_OK), f"packaged runtime is not executable: {binary}")
    return Bundle(app=app, executable=executable, bun=resources / "bin/bun")


def wait_for_cdp(
    port: int,
    process: subprocess.Popen[bytes],
    timeout: float,
) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    last_error = "CDP endpoint was not ready"
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise VisualError(f"PAD Desktop exited during startup with code {process.returncode}")
        try:
            with urllib.request.urlopen(
                f"http://127.0.0.1:{port}/json/list", timeout=1
            ) as response:
                targets = json.load(response)
            pages = [target for target in targets if target.get("type") == "page"]
            if pages and isinstance(pages[0].get("webSocketDebuggerUrl"), str):
                return pages[0]
        except (OSError, urllib.error.URLError, json.JSONDecodeError) as error:
            last_error = str(error)
        time.sleep(0.1)
    raise VisualError(f"CDP page target did not appear: {last_error}")


CDP_MATRIX_SCRIPT = r"""
const wsUrl = process.env.PAD_VISUAL_CDP_WS;
const outputDir = process.env.PAD_VISUAL_OUTPUT_DIR;
const matrix = JSON.parse(process.env.PAD_VISUAL_MATRIX || "[]");
if (!wsUrl || !outputDir || matrix.length === 0) throw new Error("missing visual matrix environment");

const socket = new WebSocket(wsUrl);
const pending = new Map();
let sequence = 0;

function call(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`CDP timeout: ${method}`));
    }, 20000);
    pending.set(id, { resolve, reject, timer, method });
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
  if (message.error) item.reject(new Error(`${item.method}: ${JSON.stringify(message.error)}`));
  else item.resolve(message.result);
});

const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

async function evaluate(expression) {
  const response = await call("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
    userGesture: true,
  });
  if (response.exceptionDetails) {
    throw new Error(`Runtime.evaluate failed: ${JSON.stringify(response.exceptionDetails)}`);
  }
  return response.result.value;
}

async function waitUntilReady() {
  const deadline = Date.now() + 15000;
  let lastAlerts = [];
  let readySamples = 0;
  while (Date.now() < deadline) {
    try {
      const result = await evaluate(String.raw`(() => {
        const shell = document.querySelector(".app-shell");
        const alerts = [...document.querySelectorAll('[role="alert"], .error-banner')]
          .filter((element) => {
            const rect = element.getBoundingClientRect();
            const style = getComputedStyle(element);
            return rect.width > 0 && rect.height > 0 && style.display !== "none" && style.visibility !== "hidden";
          })
          .map((element) => element.textContent?.trim() || "")
          .filter(Boolean);
        return {
          ready: document.readyState === "complete"
            && Boolean(shell)
            && !shell.classList.contains("is-loading")
            && !document.querySelector(".task-data-loading")
            && typeof window.padDesktop?.request === "function",
          alerts,
        };
      })()`);
      lastAlerts = Array.isArray(result?.alerts) ? result.alerts : [];
      readySamples = result?.ready ? readySamples + 1 : 0;
      if (readySamples >= 3) return result;
    } catch (error) {
      // Reload briefly destroys the execution context. Retry against the new
      // main world instead of treating that expected transition as a pass.
      readySamples = 0;
    }
    await delay(50);
  }
  return { ready: false, alerts: lastAlerts };
}

async function ensureTheme(theme) {
  const state = await evaluate(`(async () => {
    const wanted = ${JSON.stringify(theme)};
    if (typeof window.padDesktop?.request !== "function") {
      throw new Error("PAD Desktop preload request API is unavailable");
    }
    const current = await window.padDesktop.request("get_ui_state", {});
    if (!current?.state || typeof current.state !== "object") {
      throw new Error("get_ui_state returned no state");
    }
    const reloadRequired = current.state.theme !== wanted
      || document.querySelector(".app-shell")?.dataset.themePreference !== wanted;
    if (current.state.theme !== wanted) {
      await window.padDesktop.request("set_ui_state", {
        state: { ...current.state, theme: wanted },
      });
    }
    return { reloadRequired };
  })()`);
  if (state?.reloadRequired) {
    await call("Page.reload", { ignoreCache: true });
    await delay(150);
  }
  const ready = await waitUntilReady();
  if (!ready.ready) throw new Error(`renderer did not become ready for ${theme} theme`);
  if (ready.alerts.length) throw new Error(`renderer exposed alerts: ${ready.alerts.join(" | ")}`);
  const applied = await evaluate(`(() => {
    const shell = document.querySelector(".app-shell");
    return {
      rootTheme: document.documentElement.dataset.theme || null,
      shellTheme: shell?.dataset.theme || null,
      preference: shell?.dataset.themePreference || null,
    };
  })()`);
  if (applied?.rootTheme !== theme || applied?.shellTheme !== theme || applied?.preference !== theme) {
    throw new Error(`could not select ${theme} theme: ${JSON.stringify(applied)}`);
  }
}

async function ensureSidebarOpen() {
  const expression = String.raw`(async () => {
    const shell = document.querySelector(".app-shell");
    if (!shell) return false;
    if (!shell.classList.contains("sidebar-visible")) {
      const toggle = document.querySelector(".titlebar-button");
      if (!toggle) return false;
      toggle.click();
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    return Boolean(document.querySelector(".sidebar"));
  })()`;
  if (!await evaluate(expression)) throw new Error("could not expose the sidebar");
}

async function ensureSidebarClosed() {
  const expression = String.raw`(async () => {
    const shell = document.querySelector(".app-shell");
    if (!shell) return false;
    if (shell.classList.contains("sidebar-visible")) {
      const toggle = document.querySelector(".titlebar-button");
      if (!toggle) return false;
      toggle.click();
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    return shell.classList.contains("sidebar-hidden")
      && !document.querySelector(".sidebar")
      && !document.querySelector(".sidebar-backdrop");
  })()`;
  if (!await evaluate(expression)) throw new Error("could not hide the compact sidebar");
}

async function keyboardFocusProbe() {
  await evaluate(`(() => {
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    return true;
  })()`);
  await call("Input.dispatchKeyEvent", {
    type: "keyDown", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9,
  });
  await call("Input.dispatchKeyEvent", {
    type: "keyUp", key: "Tab", code: "Tab", windowsVirtualKeyCode: 9,
  });
  await delay(40);
}

const inspectExpression = String.raw`(() => {
  const visible = (element) => {
    if (!element) return false;
    const style = getComputedStyle(element);
    const rect = element.getBoundingClientRect();
    return style.display !== "none"
      && style.visibility !== "hidden"
      && Number(style.opacity || "1") > 0
      && rect.width > 0
      && rect.height > 0;
  };
  const rectValue = (element) => {
    if (!element) return null;
    const rect = element.getBoundingClientRect();
    return {
      left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom,
      width: rect.width, height: rect.height,
    };
  };
  const accessibleName = (element) => {
    const labelledBy = (element.getAttribute("aria-labelledby") || "")
      .split(/\s+/)
      .filter(Boolean)
      .map((id) => document.getElementById(id)?.textContent || "")
      .join(" ");
    const nativeLabel = "labels" in element && element.labels?.length
      ? [...element.labels].map((label) => label.textContent || "").join(" ")
      : "";
    return (
      element.getAttribute("aria-label")
      || labelledBy
      || nativeLabel
      || element.getAttribute("title")
      || element.getAttribute("placeholder")
      || element.textContent
      || ""
    ).replace(/\s+/g, " ").trim();
  };
  const isClipped = (element) => {
    const rect = element.getBoundingClientRect();
    const hit = document.elementFromPoint(
      Math.min(innerWidth - 1, Math.max(0, rect.left + rect.width / 2)),
      Math.min(innerHeight - 1, Math.max(0, rect.top + rect.height / 2)),
    );
    const occluded = !hit || (hit !== element && !element.contains(hit));
    return rect.left < -1 || rect.top < -1 || rect.right > innerWidth + 1 || rect.bottom > innerHeight + 1
      || element.scrollWidth > element.clientWidth + 1
      || element.scrollHeight > element.clientHeight + 1
      || occluded;
  };
  const styleValue = (element) => {
    if (!element) return null;
    const style = getComputedStyle(element);
    return {
      position: style.position,
      display: style.display,
      visibility: style.visibility,
      borderRadius: style.borderRadius,
      boxShadow: style.boxShadow,
      zIndex: style.zIndex,
    };
  };

  const titlebar = document.querySelector(".global-titlebar");
  const sidebar = document.querySelector('aside[aria-label*="侧边栏"], aside.sidebar');
  const workspace = document.querySelector(".workspace");
  const main = document.querySelector(".task-pane, .settings-view, .workspace-stack");
  const composerWrap = document.querySelector(".composer-wrap");
  const composer = document.querySelector(".composer");
  const backdrop = document.querySelector(".sidebar-backdrop");
  const shell = document.querySelector(".app-shell");
  const root = document.querySelector("#root");

  const interactiveSelector = [
    "button", "input", "textarea", "select", "a[href]", "[role=button]",
    "[role=treeitem]", "[role=menuitem]", "[role=tab]", "[tabindex]",
  ].join(",");
  const interactives = [...new Set([...document.querySelectorAll(interactiveSelector)])]
    .filter((element) => visible(element))
    .filter((element) => !element.matches(":disabled") && element.getAttribute("aria-disabled") !== "true")
    .filter((element) => element.getAttribute("tabindex") !== "-1");
  const unnamed = interactives
    .filter((element) => !accessibleName(element))
    .map((element) => ({ tag: element.tagName.toLowerCase(), className: element.className || "" }));

  const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const activeStyle = active ? getComputedStyle(active) : null;
  const focus = active && active !== document.body ? {
    tag: active.tagName.toLowerCase(),
    name: accessibleName(active),
    focusVisible: active.matches(":focus-visible"),
    outlineStyle: activeStyle?.outlineStyle || "",
    outlineWidth: activeStyle?.outlineWidth || "0px",
    outlineColor: activeStyle?.outlineColor || "",
  } : null;

  const actionSpecs = {
    search: ["搜索"],
    newTask: ["新任务", "新建任务"],
    settings: ["设置"],
    composer: ["任务输入", "向 Pi 描述一个任务"],
    send: ["发送", "停止任务", "重试任务"],
    sidebarToggle: ["隐藏侧边栏", "显示侧边栏"],
  };
  const actionCandidates = [...document.querySelectorAll("button, textarea, input, [role=treeitem]")]
    .filter((element) => visible(element));
  const criticalActions = Object.fromEntries(Object.entries(actionSpecs).map(([key, alternatives]) => {
    const element = actionCandidates.find((candidate) => {
      const name = accessibleName(candidate);
      return alternatives.some((text) => name.includes(text));
    });
    return [key, element ? {
      found: true,
      name: accessibleName(element),
      clipped: isClipped(element),
      rect: rectValue(element),
    } : { found: false, name: "", clipped: null, rect: null }];
  }));

  return {
    title: document.title,
    lang: document.documentElement.lang,
    theme: document.documentElement.dataset.theme || null,
    themePreference: shell?.dataset.themePreference || null,
    themeColorScheme: getComputedStyle(document.documentElement).colorScheme || null,
    viewport: {
      innerWidth, innerHeight, outerWidth, outerHeight,
      devicePixelRatio,
    },
    rects: {
      titlebar: rectValue(titlebar), sidebar: rectValue(sidebar),
      workspace: rectValue(workspace), main: rectValue(main),
      composerWrap: rectValue(composerWrap), composer: rectValue(composer),
    },
    styles: {
      sidebar: styleValue(sidebar), backdrop: styleValue(backdrop),
    },
    visible: {
      titlebar: visible(titlebar), sidebar: visible(sidebar), workspace: visible(workspace),
      main: visible(main), composerWrap: visible(composerWrap), composer: visible(composer),
      backdrop: visible(backdrop),
    },
    overflow: {
      document: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      body: document.body.scrollWidth - document.body.clientWidth,
      root: root ? root.scrollWidth - root.clientWidth : null,
      shell: shell ? shell.scrollWidth - shell.clientWidth : null,
    },
    criticalActions,
    aria: {
      interactiveCount: interactives.length,
      unnamed,
      duplicateIds: [...document.querySelectorAll("[id]")]
        .map((element) => element.id)
        .filter((id, index, values) => id && values.indexOf(id) !== index),
    },
    focus,
  };
})()`;

async function screenshot(name) {
  const result = await call("Page.captureScreenshot", {
    format: "png",
    fromSurface: true,
    captureBeyondViewport: false,
  });
  await Bun.write(`${outputDir}/${name}.png`, Buffer.from(result.data, "base64"));
}

socket.addEventListener("open", async () => {
  try {
    await call("Page.enable");
    await call("Runtime.enable");
    await call("Page.bringToFront");
    const ready = await waitUntilReady();
    if (!ready.ready) throw new Error("renderer did not reach a ready PAD shell");
    if (ready.alerts.length) throw new Error(`renderer exposed alerts: ${ready.alerts.join(" | ")}`);

    const captures = [];
    for (const item of matrix) {
      const { theme, width, height, name } = item;
      await ensureTheme(theme);
      await ensureSidebarOpen();
      await evaluate(`window.resizeTo(${Number(width)}, ${Number(height)}); true`);
      await delay(300);
      await ensureSidebarOpen();
      await keyboardFocusProbe();
      const facts = await evaluate(inspectExpression);
      await evaluate(`(() => { if (document.activeElement instanceof HTMLElement) document.activeElement.blur(); return true; })()`);
      let compactWorkspace = null;
      if (Number(width) <= 720) {
        await ensureSidebarClosed();
        await keyboardFocusProbe();
        compactWorkspace = await evaluate(inspectExpression);
        await evaluate(`(() => { if (document.activeElement instanceof HTMLElement) document.activeElement.blur(); return true; })()`);
        await ensureSidebarOpen();
        await evaluate(`(() => { if (document.activeElement instanceof HTMLElement) document.activeElement.blur(); return true; })()`);
      }
      await screenshot(name);
      captures.push({ name, requested: { theme, width, height }, facts, compactWorkspace });
    }
    process.stdout.write(JSON.stringify({ captures }));
    socket.close();
  } catch (error) {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exitCode = 1;
    socket.close();
  }
});

setTimeout(() => {
  console.error("visual matrix capture timed out");
  process.exit(1);
}, 150000).unref();
"""


CDP_CLOSE_SCRIPT = r"""
const url = process.env.PAD_VISUAL_CDP_WS;
if (!url) throw new Error("missing PAD_VISUAL_CDP_WS");
const socket = new WebSocket(url);
socket.addEventListener("open", () => socket.send(JSON.stringify({ id: 1, method: "Browser.close" })));
socket.addEventListener("message", () => { socket.close(); process.exit(0); });
setTimeout(() => process.exit(0), 1500);
"""


def run_command(
    args: list[str],
    *,
    env: dict[str, str] | None = None,
    timeout: float = 30,
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
        raise VisualError(f"command failed ({' '.join(args)}): {detail}") from error
    except subprocess.TimeoutExpired as error:
        raise VisualError(f"command timed out: {' '.join(args)}") from error


def process_table() -> dict[int, ProcessRecord]:
    output = run_command(
        ["/bin/ps", "-axo", "pid=,ppid=,pgid=,lstart=,command="], timeout=10
    ).stdout
    records: dict[int, ProcessRecord] = {}
    for line in output.splitlines():
        parts = line.strip().split(None, 8)
        if (
            len(parts) != 9
            or not parts[0].isdigit()
            or not parts[1].isdigit()
            or not parts[2].lstrip("-").isdigit()
        ):
            continue
        pid, ppid, pgid = (int(parts[index]) for index in range(3))
        records[pid] = ProcessRecord(
            pid=pid,
            ppid=ppid,
            pgid=pgid,
            started_at=" ".join(parts[3:8]),
            command=parts[8],
        )
    return records


def process_record_json(record: ProcessRecord, app: pathlib.Path, root_pid: int) -> dict[str, Any]:
    command = record.command
    resources = app / "Contents/Resources"
    if record.pid == root_pid:
        role = "electron_main"
    elif str(resources / "pad") in command:
        role = "rust_control_plane"
    elif str(app / "Contents/Frameworks") in command:
        role = "electron_helper"
    elif str(app) in command:
        role = "packaged_app_descendant"
    else:
        role = "pty_or_app_descendant"
    return {
        "pid": record.pid,
        "ppid": record.ppid,
        "pgid": record.pgid,
        "startedAt": record.started_at,
        "role": role,
        "command": command,
    }


def bundle_processes(app: pathlib.Path) -> list[ProcessRecord]:
    prefix = f"{app}/"
    return sorted(
        (record for record in process_table().values() if record.command.startswith(prefix)),
        key=lambda record: record.pid,
    )


class ProcessFamilyTracker:
    """Track and safely reap the isolated Electron -> Rust -> PTY family."""

    def __init__(self, process: subprocess.Popen[bytes], app: pathlib.Path) -> None:
        self.process = process
        self.app = app
        self.root_pid = process.pid
        try:
            self.pgid = os.getpgid(process.pid)
        except ProcessLookupError as error:
            raise VisualError("PAD Desktop exited before its process group was recorded") from error
        require(
            self.pgid == self.root_pid,
            f"isolated PAD Desktop process group mismatch: pid={self.root_pid}, pgid={self.pgid}",
        )
        self.known: dict[tuple[int, str], ProcessRecord] = {}
        self.snapshots: list[dict[str, Any]] = []

    @staticmethod
    def _identity(record: ProcessRecord) -> tuple[int, str]:
        return (record.pid, record.started_at)

    def _is_known(self, record: ProcessRecord) -> bool:
        return self._identity(record) in self.known

    def _current_family(self) -> list[ProcessRecord]:
        table = process_table()
        family = {record.pid for record in table.values() if record.pgid == self.pgid}
        family.update(
            record.pid
            for record in table.values()
            if self._identity(record) in self.known
        )
        if self.root_pid in table:
            family.add(self.root_pid)
        changed = True
        while changed:
            changed = False
            for record in table.values():
                if record.ppid in family and record.pid not in family:
                    family.add(record.pid)
                    changed = True
        records = [table[pid] for pid in sorted(family) if pid in table]
        for record in records:
            self.known[self._identity(record)] = record
        return records

    def capture(self, label: str) -> list[ProcessRecord]:
        records = self._current_family()
        self.snapshots.append(
            {
                "label": label,
                "capturedAtEpochMs": time.time_ns() // 1_000_000,
                "leaderExited": self.process.poll() is not None,
                "processes": [
                    process_record_json(record, self.app, self.root_pid) for record in records
                ],
            }
        )
        return records

    def _assert_signal_targets_safe(self, records: list[ProcessRecord]) -> None:
        protected_prefix = f"{PROTECTED_INSTALLED_APP}/"
        protected = [record.pid for record in records if protected_prefix in record.command]
        require(
            not protected,
            "refusing cleanup because the installed /Applications PAD Desktop appeared "
            f"inside the test process family: {protected}",
        )
        group_records = [record for record in records if record.pgid == self.pgid]
        if group_records:
            recognized = any(
                self._is_known(record) or str(self.app) in record.command
                for record in group_records
            )
            require(recognized, "refusing to signal an unrecognized process group")

    def _signal(self, records: list[ProcessRecord], sig: signal.Signals) -> None:
        self._assert_signal_targets_safe(records)
        if any(record.pgid == self.pgid for record in records):
            try:
                os.killpg(self.pgid, sig)
            except ProcessLookupError:
                pass
        for record in records:
            if record.pgid == self.pgid or not self._is_known(record):
                continue
            try:
                os.kill(record.pid, sig)
            except ProcessLookupError:
                pass

    def _wait_until_empty(self, timeout: float) -> list[ProcessRecord]:
        deadline = time.monotonic() + timeout
        records = self._current_family()
        while records and time.monotonic() < deadline:
            time.sleep(0.05)
            records = self._current_family()
        return records

    def cleanup(self) -> dict[str, Any]:
        records = self.capture("before_cleanup")
        if records:
            self._signal(records, signal.SIGTERM)
            records = self._wait_until_empty(4)
        if records:
            self._signal(records, signal.SIGKILL)
            records = self._wait_until_empty(2)
        try:
            self.process.wait(timeout=0.2)
        except subprocess.TimeoutExpired:
            pass
        residuals = self.capture("after_cleanup")
        evidence = self.evidence(residuals)
        require(
            not residuals,
            "PAD Desktop process family survived cleanup: "
            + ", ".join(f"{record.pid}:{record.command}" for record in residuals),
        )
        return evidence

    def evidence(self, residuals: list[ProcessRecord] | None = None) -> dict[str, Any]:
        current = residuals if residuals is not None else self._current_family()
        history = sorted(self.known.values(), key=lambda record: (record.started_at, record.pid))
        return {
            "rootPid": self.root_pid,
            "processGroupId": self.pgid,
            "testedBundle": str(self.app),
            "protectedInstalledBundle": str(PROTECTED_INSTALLED_APP),
            "snapshots": self.snapshots,
            "observedProcesses": [
                process_record_json(record, self.app, self.root_pid) for record in history
            ],
            "residualProcesses": [
                process_record_json(record, self.app, self.root_pid) for record in current
            ],
            "cleanupPassed": not current,
        }


def run_cdp_matrix(
    bundle: Bundle,
    target: dict[str, Any],
    output_dir: pathlib.Path,
    timeout: float,
) -> dict[str, Any]:
    websocket_url = target.get("webSocketDebuggerUrl")
    require(isinstance(websocket_url, str), "CDP target has no WebSocket URL")
    payload = [
        {"theme": theme, "width": width, "height": height, "name": f"{theme}-{width}x{height}"}
        for theme, width, height in MATRIX
    ]
    env = inherited_environment_allowlist()
    env.update(
        {
            "PAD_VISUAL_CDP_WS": websocket_url,
            "PAD_VISUAL_OUTPUT_DIR": str(output_dir),
            "PAD_VISUAL_MATRIX": json.dumps(payload, ensure_ascii=True),
        }
    )
    result = run_command([str(bundle.bun), "-e", CDP_MATRIX_SCRIPT], env=env, timeout=timeout)
    try:
        capture = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise VisualError(f"CDP matrix did not return JSON: {result.stdout[:500]}") from error
    require(isinstance(capture, dict), "CDP visual matrix result is not an object")
    require(len(capture.get("captures", [])) == len(MATRIX), "CDP visual matrix is incomplete")
    return capture


def close_over_cdp(bundle: Bundle, target: dict[str, Any]) -> None:
    websocket_url = target.get("webSocketDebuggerUrl")
    if not isinstance(websocket_url, str):
        return
    env = inherited_environment_allowlist()
    env["PAD_VISUAL_CDP_WS"] = websocket_url
    try:
        run_command([str(bundle.bun), "-e", CDP_CLOSE_SCRIPT], env=env, timeout=3)
    except VisualError:
        pass


def wait_for_exit(process: subprocess.Popen[bytes], timeout: float) -> bool:
    try:
        process.wait(timeout=timeout)
        return True
    except subprocess.TimeoutExpired:
        return False


def add_check(checks: list[dict[str, Any]], check_id: str, passed: bool, detail: str) -> None:
    checks.append({"id": check_id, "status": "pass" if passed else "fail", "detail": detail})


def number(value: Any) -> float:
    return float(value) if isinstance(value, (int, float)) else math.nan


def evaluate_capture(capture: dict[str, Any]) -> dict[str, Any]:
    requested = capture.get("requested", {})
    facts = capture.get("facts", {})
    width = int(requested.get("width", 0))
    height = int(requested.get("height", 0))
    theme = str(requested.get("theme", ""))
    checks: list[dict[str, Any]] = []

    viewport = facts.get("viewport", {})
    add_check(
        checks,
        "window-size",
        abs(number(viewport.get("outerWidth")) - width) <= 1
        and abs(number(viewport.get("outerHeight")) - height) <= 1,
        f"requested={width}x{height}, outer={viewport.get('outerWidth')}x{viewport.get('outerHeight')}",
    )
    add_check(
        checks,
        "theme",
        facts.get("theme") == theme
        and facts.get("themePreference") == theme
        and facts.get("themeColorScheme") == theme,
        f"requested={theme}, rendered={facts.get('theme')}, "
        f"preference={facts.get('themePreference')}, color-scheme={facts.get('themeColorScheme')}",
    )
    add_check(
        checks,
        "language",
        str(facts.get("lang", "")).lower().startswith("zh"),
        f"document lang={facts.get('lang')!r}",
    )
    add_check(
        checks,
        "title",
        facts.get("title") == "PAD Desktop",
        f"document title={facts.get('title')!r}",
    )

    visible = facts.get("visible", {})
    rects = facts.get("rects", {})
    for surface in ("titlebar", "sidebar", "main", "composerWrap", "composer"):
        rect = rects.get(surface)
        valid_rect = isinstance(rect, dict) and number(rect.get("width")) > 0 and number(rect.get("height")) > 0
        add_check(
            checks,
            f"surface-{surface}",
            visible.get(surface) is True and valid_rect,
            f"visible={visible.get(surface)!r}, rect={rect!r}",
        )

    overflow = facts.get("overflow", {})
    overflow_values = {
        key: number(value)
        for key, value in overflow.items()
        if isinstance(value, (int, float))
    }
    add_check(
        checks,
        "no-horizontal-overflow",
        bool(overflow_values) and all(value <= 1 for value in overflow_values.values()),
        f"horizontal overflow pixels={overflow_values}",
    )

    sidebar_rect = rects.get("sidebar") or {}
    workspace_rect = rects.get("workspace") or {}
    sidebar_style = facts.get("styles", {}).get("sidebar") or {}
    backdrop_visible = visible.get("backdrop") is True
    position = sidebar_style.get("position")
    gap = number(workspace_rect.get("left")) - number(sidebar_rect.get("right"))
    overlaps = number(sidebar_rect.get("right")) > number(workspace_rect.get("left")) + 2
    tiled = (
        position not in ("fixed", "absolute")
        and not backdrop_visible
        and math.isfinite(gap)
        and abs(gap) <= 2
        and sidebar_style.get("boxShadow") in ("none", "")
        and sidebar_style.get("borderRadius") in ("0px", "0px 0px 0px 0px")
    )
    overlay = position in ("fixed", "absolute") and backdrop_visible and overlaps
    if width > 960:
        layout_ok = tiled
        expected_layout = "tiled"
    elif width <= 720:
        layout_ok = overlay
        expected_layout = "overlay"
    else:
        layout_ok = tiled or overlay
        expected_layout = "valid transition layout (tiled or overlay)"
    add_check(
        checks,
        "sidebar-layout",
        layout_ok,
        f"expected={expected_layout}, position={position}, backdrop={backdrop_visible}, gap={gap}, overlap={overlaps}",
    )

    compact = capture.get("compactWorkspace") if width <= 720 else None
    if width <= 720:
        compact = compact if isinstance(compact, dict) else {}
        compact_visible = compact.get("visible", {})
        compact_rects = compact.get("rects", {})
        add_check(
            checks,
            "compact-sidebar-closed",
            compact_visible.get("sidebar") is False
            and compact_visible.get("backdrop") is False,
            f"sidebar={compact_visible.get('sidebar')!r}, backdrop={compact_visible.get('backdrop')!r}",
        )
        for surface in ("workspace", "main", "composerWrap", "composer"):
            rect = compact_rects.get(surface)
            valid_rect = (
                isinstance(rect, dict)
                and number(rect.get("width")) > 0
                and number(rect.get("height")) > 0
            )
            add_check(
                checks,
                f"compact-surface-{surface}",
                compact_visible.get(surface) is True and valid_rect,
                f"visible={compact_visible.get(surface)!r}, rect={rect!r}",
            )
        compact_overflow = {
            key: number(value)
            for key, value in compact.get("overflow", {}).items()
            if isinstance(value, (int, float))
        }
        add_check(
            checks,
            "compact-no-horizontal-overflow",
            bool(compact_overflow) and all(value <= 1 for value in compact_overflow.values()),
            f"horizontal overflow pixels={compact_overflow}",
        )
        compact_aria = compact.get("aria", {})
        add_check(
            checks,
            "compact-aria-names",
            int(compact_aria.get("interactiveCount", 0)) > 0
            and not compact_aria.get("unnamed"),
            f"interactive={compact_aria.get('interactiveCount')}, unnamed={compact_aria.get('unnamed')}",
        )

    for action in ("search", "newTask", "settings", "composer", "send", "sidebarToggle"):
        source = compact if width <= 720 and action in ("composer", "send") else facts
        evidence = source.get("criticalActions", {}).get(action, {}) if isinstance(source, dict) else {}
        add_check(
            checks,
            f"critical-action-{action}",
            evidence.get("found") is True and evidence.get("clipped") is False,
            f"name={evidence.get('name')!r}, clipped={evidence.get('clipped')!r}, rect={evidence.get('rect')!r}",
        )

    aria = facts.get("aria", {})
    add_check(
        checks,
        "aria-names",
        int(aria.get("interactiveCount", 0)) > 0 and not aria.get("unnamed"),
        f"interactive={aria.get('interactiveCount')}, unnamed={aria.get('unnamed')}",
    )
    add_check(
        checks,
        "aria-unique-ids",
        not aria.get("duplicateIds"),
        f"duplicate ids={aria.get('duplicateIds')}",
    )
    focus = facts.get("focus") or {}
    outline_width = str(focus.get("outlineWidth", "0px"))
    try:
        outline_pixels = float(outline_width.removesuffix("px"))
    except ValueError:
        outline_pixels = 0
    add_check(
        checks,
        "focus-visible",
        bool(focus.get("name"))
        and focus.get("focusVisible") is True
        and focus.get("outlineStyle") not in ("", "none")
        and outline_pixels >= 1,
        f"focus={focus}",
    )
    if width <= 720:
        compact_focus = compact.get("focus") or {}
        compact_outline_width = str(compact_focus.get("outlineWidth", "0px"))
        try:
            compact_outline_pixels = float(compact_outline_width.removesuffix("px"))
        except ValueError:
            compact_outline_pixels = 0
        add_check(
            checks,
            "compact-focus-visible",
            bool(compact_focus.get("name"))
            and compact_focus.get("focusVisible") is True
            and compact_focus.get("outlineStyle") not in ("", "none")
            and compact_outline_pixels >= 1,
            f"focus={compact_focus}",
        )

    return {
        **capture,
        "layoutMode": "tiled" if tiled else "overlay" if overlay else "invalid",
        "checks": checks,
        "status": "pass" if all(check["status"] == "pass" for check in checks) else "fail",
    }


def paeth(left: int, above: int, upper_left: int) -> int:
    estimate = left + above - upper_left
    left_distance = abs(estimate - left)
    above_distance = abs(estimate - above)
    upper_left_distance = abs(estimate - upper_left)
    if left_distance <= above_distance and left_distance <= upper_left_distance:
        return left
    if above_distance <= upper_left_distance:
        return above
    return upper_left


def read_png(path: pathlib.Path) -> PngImage:
    payload = path.read_bytes()
    require(payload.startswith(b"\x89PNG\r\n\x1a\n"), f"not a PNG image: {path}")
    offset = 8
    width = height = bit_depth = color_type = interlace = -1
    compressed = bytearray()
    while offset + 12 <= len(payload):
        length = struct.unpack(">I", payload[offset : offset + 4])[0]
        chunk_type = payload[offset + 4 : offset + 8]
        data_start = offset + 8
        data_end = data_start + length
        require(data_end + 4 <= len(payload), f"truncated PNG chunk in {path}")
        data = payload[data_start:data_end]
        if chunk_type == b"IHDR":
            width, height, bit_depth, color_type, _compression, _filter, interlace = struct.unpack(
                ">IIBBBBB", data
            )
        elif chunk_type == b"IDAT":
            compressed.extend(data)
        elif chunk_type == b"IEND":
            break
        offset = data_end + 4

    channels = {0: 1, 2: 3, 4: 2, 6: 4}.get(color_type)
    require(width > 0 and height > 0, f"PNG has no valid IHDR: {path}")
    require(bit_depth == 8 and channels is not None and interlace == 0, f"unsupported PNG encoding: {path}")
    try:
        encoded = zlib.decompress(bytes(compressed))
    except zlib.error as error:
        raise VisualError(f"could not decompress PNG: {path}") from error
    stride = width * channels
    require(len(encoded) == height * (stride + 1), f"unexpected PNG scanline size: {path}")
    decoded = bytearray(height * stride)
    source_offset = 0
    for row in range(height):
        filter_type = encoded[source_offset]
        source_offset += 1
        require(filter_type <= 4, f"unsupported PNG filter {filter_type}: {path}")
        row_start = row * stride
        previous_start = (row - 1) * stride
        for column in range(stride):
            value = encoded[source_offset + column]
            left = decoded[row_start + column - channels] if column >= channels else 0
            above = decoded[previous_start + column] if row > 0 else 0
            upper_left = (
                decoded[previous_start + column - channels]
                if row > 0 and column >= channels
                else 0
            )
            if filter_type == 1:
                value = (value + left) & 0xFF
            elif filter_type == 2:
                value = (value + above) & 0xFF
            elif filter_type == 3:
                value = (value + ((left + above) // 2)) & 0xFF
            elif filter_type == 4:
                value = (value + paeth(left, above, upper_left)) & 0xFF
            decoded[row_start + column] = value
        source_offset += stride
    return PngImage(width, height, color_type, channels, bytes(decoded))


def rgb(image: PngImage, pixel_index: int) -> tuple[int, int, int]:
    offset = pixel_index * image.bytes_per_pixel
    if image.color_type in (0, 4):
        value = image.pixels[offset]
        return value, value, value
    return image.pixels[offset], image.pixels[offset + 1], image.pixels[offset + 2]


def compare_pngs(actual_path: pathlib.Path, baseline_path: pathlib.Path, tolerance: int) -> dict[str, Any]:
    require(actual_path.is_file(), f"actual capture is missing: {actual_path}")
    require(baseline_path.is_file(), f"baseline image is missing: {baseline_path}")
    require(
        not actual_path.samefile(baseline_path),
        f"baseline aliases the generated capture and cannot be evaluated: {baseline_path}",
    )
    actual = read_png(actual_path)
    baseline = read_png(baseline_path)
    require(
        (actual.width, actual.height) == (baseline.width, baseline.height),
        f"baseline dimensions differ for {actual_path.name}: "
        f"actual {actual.width}x{actual.height}, baseline {baseline.width}x{baseline.height}",
    )
    pixel_count = actual.width * actual.height
    exact_changed = 0
    tolerance_changed = 0
    absolute_error = 0
    squared_error = 0
    for index in range(pixel_count):
        actual_rgb = rgb(actual, index)
        baseline_rgb = rgb(baseline, index)
        differences = tuple(abs(left - right) for left, right in zip(actual_rgb, baseline_rgb))
        if any(differences):
            exact_changed += 1
        if any(difference > tolerance for difference in differences):
            tolerance_changed += 1
        absolute_error += sum(differences)
        squared_error += sum(difference * difference for difference in differences)

    # Deterministic 8x8 luminance-window SSIM.  This is a real comparison of
    # the two supplied files, not a claimed comparison against Codex.
    c1 = (0.01 * 255) ** 2
    c2 = (0.03 * 255) ** 2
    ssim_total = 0.0
    windows = 0
    for top in range(0, actual.height, 8):
        for left in range(0, actual.width, 8):
            count = min(8, actual.height - top) * min(8, actual.width - left)
            sum_actual = sum_baseline = 0.0
            square_actual = square_baseline = cross = 0.0
            for y in range(top, min(top + 8, actual.height)):
                for x in range(left, min(left + 8, actual.width)):
                    index = y * actual.width + x
                    ar, ag, ab = rgb(actual, index)
                    br, bg, bb = rgb(baseline, index)
                    actual_luma = (77 * ar + 150 * ag + 29 * ab) / 256
                    baseline_luma = (77 * br + 150 * bg + 29 * bb) / 256
                    sum_actual += actual_luma
                    sum_baseline += baseline_luma
                    square_actual += actual_luma * actual_luma
                    square_baseline += baseline_luma * baseline_luma
                    cross += actual_luma * baseline_luma
            mean_actual = sum_actual / count
            mean_baseline = sum_baseline / count
            variance_actual = max(0.0, square_actual / count - mean_actual * mean_actual)
            variance_baseline = max(0.0, square_baseline / count - mean_baseline * mean_baseline)
            covariance = cross / count - mean_actual * mean_baseline
            numerator = (2 * mean_actual * mean_baseline + c1) * (2 * covariance + c2)
            denominator = (
                (mean_actual * mean_actual + mean_baseline * mean_baseline + c1)
                * (variance_actual + variance_baseline + c2)
            )
            ssim_total += numerator / denominator if denominator else 1.0
            windows += 1

    channel_values = pixel_count * 3
    return {
        "status": "evaluated",
        "actual": str(actual_path),
        "baseline": str(baseline_path),
        "dimensions": {"width": actual.width, "height": actual.height},
        "algorithm": "8x8 luminance-window SSIM; RGB pixel differences",
        "ssim": round(ssim_total / windows, 6),
        "exactPixelDifferenceRatio": round(exact_changed / pixel_count, 6),
        "tolerancePixelDifferenceRatio": round(tolerance_changed / pixel_count, 6),
        "pixelTolerance": tolerance,
        "meanAbsoluteError": round(absolute_error / channel_values, 6),
        "rootMeanSquareError": round(math.sqrt(squared_error / channel_values), 6),
    }


def apply_baselines(
    captures: list[dict[str, Any]],
    baseline_dir: pathlib.Path | None,
    output_dir: pathlib.Path,
    ssim_minimum: float,
    pixel_difference_maximum: float,
    pixel_tolerance: int,
) -> dict[str, Any]:
    if baseline_dir is None:
        for capture in captures:
            capture["baseline"] = {"status": "not_evaluated", "reason": "no --baseline-dir supplied"}
        return {
            "status": "not_evaluated",
            "reason": "no user-supplied baseline directory; no Codex SSIM is claimed",
        }

    baseline_dir = baseline_dir.expanduser().resolve()
    require(baseline_dir.is_dir(), f"baseline directory does not exist: {baseline_dir}")
    baseline_failed = False
    for capture in captures:
        name = str(capture.get("name", ""))
        actual_path = output_dir / f"{name}.png"
        baseline_path = baseline_dir / f"{name}.png"
        if not baseline_path.is_file():
            capture["baseline"] = {
                "status": "missing",
                "baseline": str(baseline_path),
            }
            capture["status"] = "fail"
            baseline_failed = True
            continue
        comparison = compare_pngs(actual_path, baseline_path, pixel_tolerance)
        comparison["thresholds"] = {
            "ssimMinimum": ssim_minimum,
            "tolerancePixelDifferenceMaximum": pixel_difference_maximum,
        }
        comparison["passed"] = (
            float(comparison["ssim"]) >= ssim_minimum
            and float(comparison["tolerancePixelDifferenceRatio"]) <= pixel_difference_maximum
        )
        capture["baseline"] = comparison
        if not comparison["passed"]:
            capture["status"] = "fail"
            baseline_failed = True
    return {
        "status": "fail" if baseline_failed else "pass",
        "directory": str(baseline_dir),
        "source": "user-supplied; this report does not assert that it is a Codex reference",
        "thresholds": {
            "ssimMinimum": ssim_minimum,
            "tolerancePixelDifferenceMaximum": pixel_difference_maximum,
            "pixelTolerance": pixel_tolerance,
        },
    }


def validate_capture_destinations(
    output_dir: pathlib.Path, baseline_dir: pathlib.Path | None
) -> None:
    resolved_output = output_dir.expanduser().resolve()
    resolved_baseline = baseline_dir.expanduser().resolve() if baseline_dir is not None else None
    require(
        resolved_baseline != resolved_output,
        "--baseline-dir must differ from --output-dir; generated captures cannot be their own baseline",
    )
    for theme, width, height in MATRIX:
        name = f"{theme}-{width}x{height}.png"
        actual_path = resolved_output / name
        require(not actual_path.is_symlink(), f"refusing symlink capture destination: {actual_path}")
        if actual_path.exists():
            require(actual_path.is_file(), f"capture destination is not a file: {actual_path}")
            require(
                actual_path.stat().st_nlink == 1,
                f"refusing multiply-linked capture destination: {actual_path}",
            )
        if resolved_baseline is None:
            continue
        baseline_path = resolved_baseline / name
        if actual_path.exists() and baseline_path.exists():
            require(
                not actual_path.samefile(baseline_path),
                f"baseline aliases generated capture: {baseline_path}",
            )


def write_report(output_dir: pathlib.Path, report: dict[str, Any]) -> pathlib.Path:
    report_path = output_dir / REPORT_NAME
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return report_path


def syntax_only() -> int:
    names = [f"{theme}-{width}x{height}" for theme, width, height in MATRIX]
    require(len(names) == 10 and len(set(names)) == 10, "visual matrix names are not unique")
    require("Page.captureScreenshot" in CDP_MATRIX_SCRIPT, "CDP screenshot action is missing")
    require("focusVisible" in CDP_MATRIX_SCRIPT, "focus-visible probe is missing")
    require('request("set_ui_state"' in CDP_MATRIX_SCRIPT, "persisted theme selection is missing")
    require("themePreference" in CDP_MATRIX_SCRIPT, "theme preference evidence is missing")
    require("themeColorScheme" in CDP_MATRIX_SCRIPT, "computed theme evidence is missing")
    require(".titlebar-theme" not in CDP_MATRIX_SCRIPT, "removed titlebar theme selector is still used")
    require("ensureSidebarClosed" in CDP_MATRIX_SCRIPT, "compact workspace probe is missing")
    require("compactWorkspace" in CDP_MATRIX_SCRIPT, "compact workspace evidence is missing")
    print(
        json.dumps(
            {
                "status": "not_evaluated",
                "verdict": "not_evaluated",
                "coverage": {
                    "layout_and_accessibility": "not_evaluated",
                    "golden_similarity": "not_evaluated",
                },
                "syntaxOnly": True,
                "matrix": names,
                "defaultApp": str(DEFAULT_APP),
                "note": "The packaged app was not launched and baseline similarity was not evaluated.",
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )
    parser.add_argument("--app", type=pathlib.Path, default=DEFAULT_APP, help="packaged PAD Desktop .app")
    parser.add_argument(
        "--output-dir",
        type=pathlib.Path,
        help="required for a real run; receives ten PNG captures, logs, and the JSON report",
    )
    parser.add_argument(
        "--baseline-dir",
        type=pathlib.Path,
        help="optional directory containing matching <theme>-<width>x<height>.png baselines",
    )
    parser.add_argument("--ssim-min", type=float, default=0.985, help="minimum SSIM when baselines are supplied")
    parser.add_argument(
        "--pixel-diff-max",
        type=float,
        default=0.05,
        help="maximum ratio of pixels exceeding --pixel-tolerance when baselines are supplied",
    )
    parser.add_argument(
        "--pixel-tolerance",
        type=int,
        default=8,
        help="per-channel RGB difference ignored by the tolerance pixel ratio",
    )
    parser.add_argument("--timeout", type=float, default=180, help="CDP matrix command timeout in seconds")
    parser.add_argument(
        "--syntax-only",
        action="store_true",
        help="validate the script/matrix without requiring or launching a complete bundle",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.syntax_only:
        return syntax_only()
    try:
        require(args.output_dir is not None, "--output-dir is required unless --syntax-only is used")
        require(0 <= args.pixel_tolerance <= 255, "--pixel-tolerance must be between 0 and 255")
        require(0 <= args.ssim_min <= 1, "--ssim-min must be between 0 and 1")
        require(0 <= args.pixel_diff_max <= 1, "--pixel-diff-max must be between 0 and 1")
    except VisualError as error:
        print(f"[FAIL] {error}", file=sys.stderr)
        return 2

    assert args.output_dir is not None
    output_dir = args.output_dir.expanduser().resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="pad-desktop-visual-state-"))
    stdout_path = output_dir / "app-stdout.log"
    stderr_path = output_dir / "app-stderr.log"
    report: dict[str, Any] = {
        "schemaVersion": 2,
        "generatedAt": dt.datetime.now(dt.timezone.utc).isoformat(),
        "app": str(args.app.expanduser().resolve()),
        "outputDirectory": str(output_dir),
        "matrix": [],
        "baseline": {"status": "not_evaluated"},
        "coverage": {
            "layout_and_accessibility": "not_evaluated",
            "golden_similarity": "not_evaluated",
        },
        "verdict": "fail",
        "localGatePassed": False,
        "completeGoldenEvaluation": False,
        "completeBaselineEvaluation": False,
        "status": "fail",
    }
    process: subprocess.Popen[bytes] | None = None
    tracker: ProcessFamilyTracker | None = None
    cleanup_done = False
    target: dict[str, Any] | None = None
    try:
        validate_capture_destinations(output_dir, args.baseline_dir)
        bundle = check_bundle(args.app)
        prelaunch = bundle_processes(bundle.app)
        require(
            not prelaunch,
            "another process from the packaged test bundle is already running: "
            + ", ".join(str(record.pid) for record in prelaunch),
        )
        for directory in (scratch / "home", scratch / "data", scratch / "user-data"):
            directory.mkdir(mode=0o700)
        port = free_tcp_port()
        env = safe_environment(scratch / "home", scratch / "data")
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            process = subprocess.Popen(
                [
                    str(bundle.executable),
                    f"--remote-debugging-port={port}",
                    "--remote-allow-origins=*",
                    f"--user-data-dir={scratch / 'user-data'}",
                    "--no-first-run",
                    "--disable-default-apps",
                    "--window-size=1280,820",
                ],
                env=env,
                stdout=stdout,
                stderr=stderr,
                start_new_session=True,
            )
        tracker = ProcessFamilyTracker(process, bundle.app)
        tracker.capture("launched")
        target = wait_for_cdp(port, process, min(args.timeout, 30))
        tracker.capture("cdp_ready")
        captured = run_cdp_matrix(bundle, target, output_dir, args.timeout)
        tracker.capture("matrix_captured")
        evaluated = [evaluate_capture(item) for item in captured["captures"]]
        layout_passed = all(item["status"] == "pass" for item in evaluated)
        report["matrix"] = evaluated
        report["baseline"] = apply_baselines(
            evaluated,
            args.baseline_dir,
            output_dir,
            args.ssim_min,
            args.pixel_diff_max,
            args.pixel_tolerance,
        )
        baseline_status = str(report["baseline"].get("status", "not_evaluated"))
        report["coverage"] = {
            "layout_and_accessibility": "pass" if layout_passed else "fail",
            "golden_similarity": baseline_status,
        }
        if not layout_passed or baseline_status == "fail":
            report["status"] = "fail"
            report["verdict"] = "fail"
            report["localGatePassed"] = False
        elif baseline_status == "pass":
            report["status"] = "pass"
            report["verdict"] = "pass"
            report["localGatePassed"] = True
            report["completeBaselineEvaluation"] = True
        else:
            # Layout, localization and accessibility are still useful local
            # gates, but a run without a supplied baseline is not a complete
            # visual Golden pass and must never be reported as one.
            report["status"] = "partial"
            report["verdict"] = "partial"
            report["localGatePassed"] = True
        if target is not None:
            close_over_cdp(bundle, target)
        if not wait_for_exit(process, 10):
            raise VisualError("PAD Desktop did not exit after CDP Browser.close")
        require(process.returncode == 0, f"PAD Desktop exited with code {process.returncode}")
        report["processFamily"] = tracker.cleanup()
        cleanup_done = True
        (output_dir / PROCESS_REPORT_NAME).write_text(
            json.dumps(report["processFamily"], ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        report_path = write_report(output_dir, report)
        if report["verdict"] == "fail":
            failed = [item["name"] for item in evaluated if item["status"] != "pass"]
            baseline_detail = (
                "baseline comparison failed"
                if report["baseline"].get("status") == "fail"
                else "layout/accessibility checks failed"
            )
            names = ", ".join(failed) if failed else "baseline coverage"
            raise VisualError(f"{baseline_detail}: {names}")
        print(f"[PASS] layout/accessibility matrix: {len(evaluated)} captures")
        print(f"[PASS] report: {report_path}")
        if report["baseline"]["status"] == "not_evaluated":
            print("[PARTIAL] Golden similarity not evaluated; no baseline supplied and no Codex SSIM is claimed")
        else:
            print(f"[PASS] complete user-supplied baseline comparison: {report['baseline']['directory']}")
        return 0
    except (VisualError, OSError, ValueError, zlib.error) as error:
        if tracker is not None and not cleanup_done:
            try:
                report["processFamily"] = tracker.cleanup()
                cleanup_done = True
            except (VisualError, OSError) as cleanup_error:
                report["processFamily"] = tracker.evidence()
                error = VisualError(f"{error}; process cleanup failed: {cleanup_error}")
            (output_dir / PROCESS_REPORT_NAME).write_text(
                json.dumps(report["processFamily"], ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
        report["status"] = "fail"
        report["verdict"] = "fail"
        report["localGatePassed"] = False
        report["fatalError"] = str(error)
        report_path = write_report(output_dir, report)
        print(f"[FAIL] {error}", file=sys.stderr)
        print(f"[INFO] report retained at: {report_path}", file=sys.stderr)
        return 1
    finally:
        if tracker is not None and not cleanup_done:
            try:
                tracker.cleanup()
            except (VisualError, OSError):
                pass
        shutil.rmtree(scratch, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
