#!/usr/bin/env python3
"""Final black-box acceptance test for the packaged macOS Electron app.

This test never imports renderer or Electron implementation code.  It launches
the packaged application with an isolated HOME, Chromium user-data directory,
and PAD_DESKTOP_DATA_DIR, then inspects the visible renderer over the local
Chrome DevTools Protocol endpoint.  Codex/ChatGPT isolation is proven with
controlled sentinel trees in the isolated test filesystem, including a custom
CODEX_HOME outside the synthetic HOME; the user's real product data is never
traversed, opened, hashed, or written.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import platform
import re
import secrets
import shutil
import signal
import socket
import sqlite3
import stat
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any, Iterable


ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_APP = (
    ROOT
    / "apps/pad-desktop/out/PAD Desktop-darwin-arm64/PAD Desktop.app"
)
EXPECTED_PRODUCT_NAME = "PAD Desktop"
EXPECTED_BUNDLE_ID = "cn.ghostcloud.pad.desktop"
EXPECTED_MINIMUM_MACOS = "13.0"
EXPECTED_PROTOCOL_MINIMUM = 2
PROCESS_EXIT_TIMEOUT_SECONDS = 12.0


class AcceptanceError(RuntimeError):
    """A user-visible packaged-app acceptance failure."""


@dataclass(frozen=True)
class Bundle:
    app: pathlib.Path
    executable: pathlib.Path
    pad: pathlib.Path
    bun: pathlib.Path
    pi: pathlib.Path
    version: str
    bundle_id: str


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AcceptanceError(message)


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
        command = " ".join(args)
        detail = (error.stderr or error.stdout or "").strip()
        raise AcceptanceError(f"command failed ({command}): {detail}") from error
    except subprocess.TimeoutExpired as error:
        raise AcceptanceError(f"command timed out: {' '.join(args)}") from error


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
        }
    )
    return env


def plist_string(plist: dict[str, Any], key: str) -> str:
    value = plist.get(key)
    require(isinstance(value, str) and bool(value.strip()), f"Info.plist missing {key}")
    return value.strip()


def executable_architectures(path: pathlib.Path) -> set[str]:
    output = run_command(["/usr/bin/lipo", "-archs", str(path)]).stdout.strip()
    return set(output.split())


def executable_version(path: pathlib.Path, env: dict[str, str]) -> str:
    result = run_command([str(path), "--version"], env=env)
    output = (result.stdout or result.stderr).strip()
    require(bool(output), f"{path.name} --version returned no version")
    return output


def extract_semver(value: str, label: str) -> str:
    match = re.search(r"(?<!\d)(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)", value)
    require(match is not None, f"{label} did not report a semantic version: {value!r}")
    return match.group(1)


def check_bundle(app_argument: pathlib.Path, scratch: pathlib.Path) -> Bundle:
    require(platform.system() == "Darwin", "Electron app acceptance is macOS-only")
    require(platform.machine() == "arm64", f"expected arm64 host, found {platform.machine()}")

    app = app_argument.expanduser().resolve()
    require(app == DEFAULT_APP.resolve(), f"final app must be tested at {DEFAULT_APP}, found {app}")
    require(app.name == f"{EXPECTED_PRODUCT_NAME}.app", f"unexpected app product path: {app}")
    plist_path = app / "Contents/Info.plist"
    require(plist_path.is_file(), f"missing packaged Info.plist: {plist_path}")
    plist_result = run_command(
        ["/usr/bin/plutil", "-convert", "json", "-o", "-", str(plist_path)]
    )
    try:
        plist = json.loads(plist_result.stdout)
    except json.JSONDecodeError as error:
        raise AcceptanceError("Info.plist could not be converted to JSON") from error
    require(isinstance(plist, dict), "Info.plist root is not a dictionary")

    executable_name = plist_string(plist, "CFBundleExecutable")
    require(executable_name == "PADDesktop", f"unexpected CFBundleExecutable: {executable_name}")
    for key in ("CFBundleName", "CFBundleDisplayName"):
        require(plist_string(plist, key) == EXPECTED_PRODUCT_NAME, f"unexpected {key}")
    bundle_id = plist_string(plist, "CFBundleIdentifier")
    require(bundle_id == EXPECTED_BUNDLE_ID, f"unexpected bundle identifier: {bundle_id}")
    version = plist_string(plist, "CFBundleShortVersionString")
    require(
        plist_string(plist, "LSMinimumSystemVersion") == EXPECTED_MINIMUM_MACOS,
        f"LSMinimumSystemVersion must be {EXPECTED_MINIMUM_MACOS}",
    )

    tcc_keys = sorted(
        key
        for key in plist
        if isinstance(key, str) and key.startswith("NS") and key.endswith("UsageDescription")
    )
    require(not tcc_keys, f"unnecessary TCC usage descriptions present: {', '.join(tcc_keys)}")
    require("NSAppTransportSecurity" not in plist, "unnecessary ATS override is present")

    icon_name = plist_string(plist, "CFBundleIconFile")
    icon_name = icon_name if pathlib.Path(icon_name).suffix else f"{icon_name}.icns"
    icon = app / "Contents/Resources" / icon_name
    require(icon.name == "PADDesktop.icns", f"unexpected application icon: {icon.name}")
    require(icon.is_file() and icon.stat().st_size > 0, f"missing packaged icon: {icon}")

    executable = app / "Contents/MacOS" / executable_name
    pad = app / "Contents/Resources/pad"
    bun = app / "Contents/Resources/bin/bun"
    pi = app / "Contents/Resources/bin/pi"
    node_shim = app / "Contents/Resources/bin/node"
    pi_package = app / "Contents/Resources/pi/package.json"
    for member in (executable, pad, bun, pi, node_shim):
        require(member.is_file() and os.access(member, os.X_OK), f"missing executable: {member}")
    require(pi_package.is_file(), f"missing bundled Pi package metadata: {pi_package}")

    for member in (executable, pad, bun):
        architectures = executable_architectures(member)
        require(architectures == {"arm64"}, f"{member.name} architectures are {sorted(architectures)}")

    run_command(["/usr/bin/plutil", "-lint", str(plist_path)])
    fuse_cli = ROOT / "apps/pad-desktop/node_modules/.bin/electron-fuses"
    require(fuse_cli.is_file() and os.access(fuse_cli, os.X_OK), "electron-fuses CLI is unavailable")
    fuse_output = run_command([str(fuse_cli), "read", "--app", str(app)]).stdout
    for expected in (
        "RunAsNode is Disabled",
        "EnableCookieEncryption is Disabled",
        "EnableNodeOptionsEnvironmentVariable is Disabled",
        "EnableNodeCliInspectArguments is Disabled",
        "EnableEmbeddedAsarIntegrityValidation is Enabled",
        "OnlyLoadAppFromAsar is Enabled",
    ):
        require(expected in fuse_output, f"Electron fuse is not hardened: expected {expected!r}")

    # Fuse writes mutate the Mach-O binary, so signature verification belongs
    # after the final fuse state has been read from the packaged executable.
    run_command(["/usr/bin/codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)])
    signature = run_command(
        ["/usr/bin/codesign", "-d", "--verbose=4", str(app)]
    ).stderr
    require(f"Identifier={bundle_id}" in signature, "codesign identifier does not match Info.plist")

    isolated_home = scratch / "bundle-check-home"
    isolated_data = scratch / "bundle-check-data"
    isolated_home.mkdir(mode=0o700)
    isolated_data.mkdir(mode=0o700)
    env = safe_environment(isolated_home, isolated_data)
    pad_version = extract_semver(executable_version(pad, env), "pad")
    bun_version = extract_semver(executable_version(bun, env), "Bun")
    pi_version = extract_semver(executable_version(pi, env), "Pi")
    require(pad_version == version, f"pad {pad_version} does not match app version {version}")
    require(bool(bun_version), "bundled Bun version is empty")
    with pi_package.open("r", encoding="utf-8") as handle:
        pi_metadata = json.load(handle)
    package_version = str(pi_metadata.get("version", ""))
    require(pi_version == package_version, f"Pi wrapper {pi_version} != package {package_version}")

    print(
        f"[PASS] bundle arm64, signed, {bundle_id}, macOS {EXPECTED_MINIMUM_MACOS}+, "
        f"pad {pad_version}, Bun {bun_version}, Pi {pi_version}"
    )
    return Bundle(app, executable, pad, bun, pi, version, bundle_id)


def protected_locations(
    home: pathlib.Path, custom_codex_home: pathlib.Path | None = None
) -> list[tuple[str, pathlib.Path]]:
    locations = [
        ("codex-home", home / ".codex"),
        ("chatgpt-home-legacy", home / ".chatgpt"),
        ("codex-container", home / "Library/Containers/com.openai.codex"),
        ("chatgpt-container", home / "Library/Containers/com.openai.chat"),
        ("chatgpt-container-legacy", home / "Library/Containers/com.openai.chatgpt"),
        ("codex-group-container", home / "Library/Group Containers/group.com.openai.codex"),
        (
            "codex-notifications-group-container",
            home / "Library/Group Containers/2DC432GLL2.com.openai.codex.notifications",
        ),
        (
            "chatgpt-cua-service-group-container",
            home / "Library/Group Containers/2DC432GLL2.com.openai.sky.CUAService",
        ),
        ("chatgpt-group-container", home / "Library/Group Containers/group.com.openai.chat"),
        (
            "chatgpt-group-container-legacy",
            home / "Library/Group Containers/group.com.openai.chatgpt",
        ),
        ("codex-support", home / "Library/Application Support/com.openai.codex"),
        ("codex-support-current", home / "Library/Application Support/Codex"),
        ("openai-support", home / "Library/Application Support/OpenAI"),
        ("chatgpt-support", home / "Library/Application Support/com.openai.chat"),
        (
            "chatgpt-support-legacy",
            home / "Library/Application Support/com.openai.chatgpt",
        ),
        ("chatgpt-support-current", home / "Library/Application Support/ChatGPT"),
        ("codex-cache", home / "Library/Caches/Codex"),
        ("codex-cache-bundle", home / "Library/Caches/com.openai.codex"),
        ("chatgpt-cache", home / "Library/Caches/ChatGPT"),
        ("chatgpt-cache-bundle", home / "Library/Caches/com.openai.chat"),
        ("chatgpt-cache-bundle-legacy", home / "Library/Caches/com.openai.chatgpt"),
        ("codex-logs", home / "Library/Logs/com.openai.codex"),
        ("chatgpt-logs", home / "Library/Logs/com.openai.chat"),
        ("chatgpt-logs-legacy", home / "Library/Logs/com.openai.chatgpt"),
        ("codex-http-storage", home / "Library/HTTPStorages/com.openai.codex"),
        (
            "codex-http-cookie-storage",
            home / "Library/HTTPStorages/com.openai.codex.binarycookies",
        ),
        ("chatgpt-http-storage", home / "Library/HTTPStorages/com.openai.chat"),
        (
            "chatgpt-http-cookie-storage",
            home / "Library/HTTPStorages/com.openai.chat.binarycookies",
        ),
        ("chatgpt-http-storage-legacy", home / "Library/HTTPStorages/com.openai.chatgpt"),
        (
            "chatgpt-http-cookie-storage-legacy",
            home / "Library/HTTPStorages/com.openai.chatgpt.binarycookies",
        ),
        ("codex-preferences", home / "Library/Preferences/com.openai.codex.plist"),
        ("chatgpt-preferences", home / "Library/Preferences/com.openai.chat.plist"),
        (
            "chatgpt-preferences-legacy",
            home / "Library/Preferences/com.openai.chatgpt.plist",
        ),
    ]
    if custom_codex_home is not None:
        locations.append(("codex-custom-home", custom_codex_home))
    return locations


def metadata_digest(root: pathlib.Path) -> dict[str, Any]:
    """Hash names and lstat fields without opening any file content."""

    digest = hashlib.sha256()
    count = 0
    error_count = 0
    if not root.exists() and not root.is_symlink():
        return {"exists": False, "entries": 0, "sha256": digest.hexdigest(), "errors": 0}

    pending = [root]
    while pending:
        path = pending.pop()
        try:
            metadata = path.lstat()
        except OSError:
            error_count += 1
            continue
        try:
            relative = "." if path == root else str(path.relative_to(root))
        except ValueError:
            relative = str(path)
        fields = (
            relative,
            metadata.st_mode,
            metadata.st_uid,
            metadata.st_gid,
            metadata.st_ino,
            metadata.st_size,
            metadata.st_mtime_ns,
            metadata.st_ctime_ns,
        )
        digest.update((json.dumps(fields, ensure_ascii=False) + "\n").encode())
        count += 1
        if stat.S_ISDIR(metadata.st_mode) and not stat.S_ISLNK(metadata.st_mode):
            try:
                children = sorted(path.iterdir(), key=lambda child: child.name, reverse=True)
            except OSError:
                error_count += 1
                continue
            pending.extend(children)
    return {
        "exists": True,
        "entries": count,
        "sha256": digest.hexdigest(),
        "errors": error_count,
    }


def snapshot_protected_metadata(
    locations: Iterable[tuple[str, pathlib.Path]],
) -> dict[str, dict[str, Any]]:
    # Only stable labels and aggregate digests are persisted.  Relative names
    # contribute to the digest in memory but never appear in an artifact.
    return {label: metadata_digest(path) for label, path in locations}


def free_tcp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def process_table() -> dict[int, tuple[int, str]]:
    output = run_command(["/bin/ps", "-axo", "pid=,ppid=,command="]).stdout
    result: dict[int, tuple[int, str]] = {}
    for line in output.splitlines():
        match = re.match(r"\s*(\d+)\s+(\d+)\s+(.*)", line)
        if match:
            result[int(match.group(1))] = (int(match.group(2)), match.group(3))
    return result


def descendants(root_pid: int, table: dict[int, tuple[int, str]]) -> set[int]:
    found = {root_pid}
    changed = True
    while changed:
        changed = False
        for pid, (parent, _command) in table.items():
            if parent in found and pid not in found:
                found.add(pid)
                changed = True
    return found


def matching_processes(needles: Iterable[str]) -> list[tuple[int, str]]:
    expected = tuple(needles)
    return [
        (pid, command)
        for pid, (_parent, command) in process_table().items()
        if any(needle in command for needle in expected)
    ]


def is_test_bundle_process(command: str, app: pathlib.Path) -> bool:
    """Return true only when the process executable lives inside this test bundle."""

    return command.startswith(f"{app}{os.sep}")


def create_protected_sentinels(
    home: pathlib.Path, sentinel: str, custom_codex_home: pathlib.Path | None = None
) -> dict[str, dict[str, Any]]:
    """Create test-owned lookalike namespaces without touching real product data."""

    locations = protected_locations(home, custom_codex_home)
    for label, root in locations:
        root.mkdir(parents=True, exist_ok=True, mode=0o700)
        marker = root / f"pad-must-not-touch-{label}.sentinel"
        marker.write_text(sentinel, encoding="utf-8")
        marker.chmod(0o600)
    return snapshot_protected_metadata(locations)


def assert_protected_sentinel_contents(
    home: pathlib.Path, sentinel: str, custom_codex_home: pathlib.Path | None = None
) -> None:
    """Read only marker files created in the isolated test filesystem."""

    for label, root in protected_locations(home, custom_codex_home):
        marker = root / f"pad-must-not-touch-{label}.sentinel"
        require(marker.read_text(encoding="utf-8") == sentinel, f"protected sentinel changed: {label}")


def assert_complete_protected_snapshot(
    snapshot: dict[str, dict[str, Any]], phase: str
) -> None:
    incomplete = [
        label
        for label, value in snapshot.items()
        if value.get("exists") is not True
        or int(value.get("entries", 0)) < 2
        or int(value.get("errors", 0)) != 0
    ]
    require(not incomplete, f"{phase} protected sentinel snapshot is incomplete: {incomplete}")


def assert_protected_data_roots_fail_closed(
    bundle: Bundle,
    home: pathlib.Path,
    custom_codex_home: pathlib.Path,
    sentinel: str,
    artifact_dir: pathlib.Path,
) -> None:
    """Prove every synthetic provider namespace is rejected before mutation."""

    protected = protected_locations(home, custom_codex_home)
    results: list[dict[str, Any]] = []
    result_path = artifact_dir / "protected-data-root-denials.json"
    for label, candidate in protected:
        before = snapshot_protected_metadata(protected)
        assert_complete_protected_snapshot(before, f"before-{label}-denial")
        env = safe_environment(home, candidate)
        # This direct sidecar launch models Electron's protection-only mapping;
        # CODEX_HOME itself is intentionally absent from the Rust/Pi environment.
        env["PAD_PROTECTED_CODEX_HOME"] = str(custom_codex_home)
        process = subprocess.Popen(
            [str(bundle.pad), "__internal", "desktop-server"],
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            start_new_session=True,
        )
        try:
            try:
                stdout, stderr = process.communicate(input=b"", timeout=5)
            except subprocess.TimeoutExpired as error:
                raise AcceptanceError(
                    f"protected data-root launch did not fail closed: {label}"
                ) from error
        finally:
            cleanup_process_group(process)

        after = snapshot_protected_metadata(protected)
        assert_complete_protected_snapshot(after, f"after-{label}-denial")
        assert_protected_sentinel_contents(home, sentinel, custom_codex_home)
        changed = [name for name in before if before[name] != after[name]]
        diagnostic = stderr.decode("utf-8", errors="replace")
        result = {
            "label": label,
            "returnCode": process.returncode,
            "stdoutEmpty": not stdout.strip(),
            "expectedDiagnostic": "refusing unsafe PAD Desktop data root" in diagnostic,
            "metadataUnchanged": not changed,
        }
        results.append(result)
        result_path.write_text(
            json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        require(not changed, f"protected data-root denial mutated sentinels: {changed}")
        require(process.returncode not in (None, 0), f"protected data root was accepted: {label}")
        require(not stdout.strip(), f"protected data-root denial wrote bridge output: {label}")
        require(
            result["expectedDiagnostic"],
            f"protected data-root denial had an unexpected failure: {label}",
        )

    print(f"[PASS] all {len(results)} protected data-root overrides failed closed before mutation")


def assert_processes_hold_no_protected_descriptors(
    process_ids: Iterable[int], protected: Iterable[tuple[str, pathlib.Path]]
) -> None:
    """Inspect our own processes' live descriptors, never the protected trees."""

    ids = sorted(set(process_ids))
    require(ids, "PAD Desktop process family disappeared before isolation inspection")
    lsof = pathlib.Path("/usr/sbin/lsof")
    require(lsof.is_file(), "lsof is unavailable for protected-path isolation inspection")
    result = run_command([str(lsof), "-Fn", "-p", ",".join(str(pid) for pid in ids)])
    open_files = result.stdout
    opened = [label for label, root in protected if str(root) in open_files]
    require(not opened, f"PAD Desktop opened protected sentinel namespaces: {opened}")


def wait_for_cdp(port: int, process: subprocess.Popen[bytes], timeout: float = 20.0) -> dict[str, Any]:
    deadline = time.monotonic() + timeout
    last_error = "CDP endpoint was not ready"
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise AcceptanceError(f"PAD Desktop exited during startup with code {process.returncode}")
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/list", timeout=1) as response:
                targets = json.load(response)
            pages = [target for target in targets if target.get("type") == "page"]
            if pages and isinstance(pages[0].get("webSocketDebuggerUrl"), str):
                return pages[0]
        except (OSError, urllib.error.URLError, json.JSONDecodeError) as error:
            last_error = str(error)
        time.sleep(0.1)
    raise AcceptanceError(f"CDP page target did not appear: {last_error}")


CDP_CAPTURE_SCRIPT = r"""
const wsUrl = process.env.PAD_CDP_WS;
const artifactDir = process.env.PAD_CDP_ARTIFACT_DIR;
const sentinel = process.env.PAD_E2E_SENTINEL;
const workspace = process.env.PAD_E2E_WORKSPACE;
const testRun = process.env.PAD_E2E_TEST_RUN;
const ptyMarker = process.env.PAD_E2E_PTY_MARKER;
if (!wsUrl || !artifactDir || !sentinel || !workspace || !testRun || !ptyMarker) {
  throw new Error("missing CDP environment");
}

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

async function evaluateAfterReload(expression) {
  let lastError = null;
  for (let attempt = 0; attempt < 120; attempt += 1) {
    try {
      return await evaluate(expression);
    } catch (error) {
      lastError = error;
      const message = String(error?.message ?? error);
      if (!/execution context|cannot find context|navigat|target closed/i.test(message)) throw error;
      await delay(50);
    }
  }
  throw lastError ?? new Error("renderer execution context did not return after reload");
}

const workflowExpression = String.raw`(async () => {
  const sentinel = ${JSON.stringify(sentinel)};
  const e2eWorkspace = ${JSON.stringify(workspace)};
  const testRun = ${JSON.stringify(testRun)};
  const ptyMarker = ${JSON.stringify(ptyMarker)};
  const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
  let ready = false;
  let readySamples = 0;
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const shell = document.querySelector(".app-shell");
    const candidate = document.readyState === "complete"
      && Boolean(shell)
      && !shell.classList.contains("is-loading")
      && !document.querySelector(".task-data-loading")
      && typeof window.padDesktop?.bootstrap === "function";
    readySamples = candidate ? readySamples + 1 : 0;
    if (readySamples >= 3) { ready = true; break; }
    await delay(50);
  }
  if (!ready) throw new Error("renderer did not reach a ready PAD shell");

  const request = (action, params = {}) => window.padDesktop.request(action, params);
  const bootstrap = () => window.padDesktop.bootstrap();
  const requiredString = (value, label) => {
    if (typeof value !== "string" || value.length === 0) throw new Error("missing " + label);
    return value;
  };
  const switchProfile = async (profileId, selectedTaskId = null) => {
    const current = await request("get_ui_state", {});
    const state = current?.state;
    if (!state || typeof state !== "object") throw new Error("get_ui_state returned no state");
    await request("set_ui_state", {
      state: {
        ...state,
        active_profile_id: profileId,
        selected_task_id: selectedTaskId,
        collapsed_project_ids: [],
      },
    });
    const loaded = await bootstrap();
    if (loaded?.profile?.id !== profileId || loaded?.ui_state?.active_profile_id !== profileId) {
      throw new Error("profile switch did not become authoritative");
    }
    return loaded;
  };
  const jsonContainsAny = (value, tokens) => {
    const text = JSON.stringify(value);
    return tokens.some((token) => typeof token === "string" && token.length > 0 && text.includes(token));
  };
  const tokenHits = (values, tokens) => {
    const text = values.map((value) => JSON.stringify(value)).join("\n");
    return tokens.filter((token) => typeof token === "string" && token.length > 0 && text.includes(token)).length;
  };

  const profileA = "profile-e2e-a-" + testRun;
  const profileB = "profile-e2e-b-" + testRun;
  const workspaceA = e2eWorkspace + "/profile-a";
  const workspaceB = e2eWorkspace + "/profile-b";
  const taskARequested = "task-e2e-a-" + testRun;
  const taskBRequested = "task-e2e-b-" + testRun;
  const aProjectName = "A_PROJECT_" + testRun;
  const bProjectName = "B_PRIVATE_PROJECT_" + testRun;
  const aTitle = "甲账号任务-" + testRun;
  const bTitle = "B_PRIVATE_TITLE_" + testRun;
  const bSummary = "B_PRIVATE_SUMMARY_" + testRun;

  await request("create_profile", {
    profile_id: profileA,
    name: "验收账号甲-" + testRun,
    permission_mode: "system_full",
    unattended: true,
  });
  await switchProfile(profileA);
  const projectAResult = await request("create_project", {
    profile_id: profileA,
    name: aProjectName,
    cwd: workspaceA,
  });
  const projectA = requiredString(projectAResult?.project?.id, "profile A project id");
  const taskAResult = await request("create_task", {
    task_id: taskARequested,
    project_id: projectA,
    profile_id: profileA,
    title: aTitle,
    summary: "A_SUMMARY_" + testRun,
    cwd: workspaceA,
    environment: "local",
  });
  const taskA = requiredString(taskAResult?.task?.id, "profile A task id");
  const completeA = await bootstrap();

  await request("create_profile", {
    profile_id: profileB,
    name: "验收账号乙-" + testRun,
    permission_mode: "system_full",
    unattended: true,
  });
  await switchProfile(profileB);
  const projectBResult = await request("create_project", {
    profile_id: profileB,
    name: bProjectName,
    cwd: workspaceB,
  });
  const projectB = requiredString(projectBResult?.project?.id, "profile B project id");
  const taskBResult = await request("create_task", {
    task_id: taskBRequested,
    project_id: projectB,
    profile_id: profileB,
    title: bTitle,
    summary: bSummary,
    cwd: workspaceB,
    environment: "local",
  });
  const taskB = requiredString(taskBResult?.task?.id, "profile B task id");
  const bootstrapB = await switchProfile(profileB, taskB);

  const aRecordTokens = [
    taskA,
    projectA,
    aProjectName,
    aTitle,
    workspaceA,
    ...((completeA?.records?.projects ?? []).map((project) => project?.id)),
    ...((completeA?.records?.tasks ?? []).map((task) => task?.id)),
  ].filter(Boolean);
  const bRecordTokens = [
    taskB,
    projectB,
    bProjectName,
    bTitle,
    bSummary,
    workspaceB,
    ...((bootstrapB?.records?.projects ?? []).map((project) => project?.id)),
    ...((bootstrapB?.records?.tasks ?? []).map((task) => task?.id)),
  ].filter(Boolean);
  const bOwnRecordsVisible = (bootstrapB?.records?.projects ?? []).some((project) => project?.id === projectB)
    && (bootstrapB?.records?.tasks ?? []).some((task) => task?.id === taskB);
  const aLeaksWhileB = tokenHits([bootstrapB?.records, bootstrapB?.sidebar], aRecordTokens);

  const bootstrapA = await switchProfile(profileA, taskA);
  const sidebarA = await request("list_sidebar", {});
  const bLeaksWhileA = tokenHits(
    [bootstrapA?.records, bootstrapA?.sidebar, sidebarA?.records, sidebarA?.sidebar],
    bRecordTokens,
  );
  const aOwnRecordsVisible = (bootstrapA?.records?.projects ?? []).some((project) => project?.id === projectA)
    && (bootstrapA?.records?.tasks ?? []).some((task) => task?.id === taskA);

  const absolutePath = /(?:^|[\s("'])\/(?:Users|private|tmp|var\/folders|Volumes|Applications)\//i;
  const unexpectedPaneIds = [];
  let unexpectedTaskRuntime = false;
  const attemptDenied = async (action, params, sensitiveTokens = []) => {
    try {
      const value = await request(action, params);
      if (action === "terminal_open" && typeof value?.pane_id === "string") {
        unexpectedPaneIds.push(value.pane_id);
      }
      if (action === "start_task" || action === "retry_task" || action === "prompt") {
        unexpectedTaskRuntime = true;
      }
      return { action, rejected: false, boundary: false, leak: false, message: "unexpected success" };
    } catch (error) {
      const message = String(error?.message ?? error);
      const knownLeak = [sentinel, ptyMarker, ...sensitiveTokens]
        .some((token) => token && message.includes(String(token)));
      const pathLeak = absolutePath.test(message);
      const leak = knownLeak || pathLeak;
      return {
        action,
        rejected: true,
        boundary: /unavailable for the active profile/i.test(message),
        leak,
        message: leak ? "[redacted: unsafe error disclosure]" : message.slice(0, 240),
      };
    }
  };

  const bSensitive = [taskB, projectB, bProjectName, bTitle, bSummary, workspaceB, ptyMarker];
  const deniedTaskActions = [];
  for (const [action, params] of [
    ["history", { task_id: taskB }],
    ["start_task", { task_id: taskB }],
    ["prompt", { task_id: taskB, prompt: "B_PRIVATE_PROMPT_" + testRun }],
    ["poll", { task_id: taskB }],
    ["abort", { task_id: taskB }],
    ["stop", { task_id: taskB }],
    ["stop_task", { task_id: taskB }],
    ["retry_task", { task_id: taskB }],
    ["get_messages", { task_id: taskB }],
    ["get_state", { task_id: taskB }],
    ["get_entries", { task_id: taskB }],
    ["set_task", { task_id: taskB, unread: true }],
    ["set_model", { task_id: taskB, provider: "openai", model: "e2e-model" }],
    ["set_thinking_level", { task_id: taskB, thinking_level: "low" }],
    ["respond_ui", { task_id: taskB, request_id: "e2e-" + testRun, value: "deny" }],
    ["extension_ui_response", { task_id: taskB, request_id: "e2e-" + testRun, value: "deny" }],
    ["runtime_snapshot", { task_id: taskB }],
    ["terminal_open", { task_id: taskB, columns: 80, rows: 24 }],
  ]) {
    deniedTaskActions.push(await attemptDenied(action, params, bSensitive));
  }

  const deniedScopedActions = [];
  for (const [action, params] of [
    ["provider_status", { profile_id: profileB }],
    ["set_profile", { profile_id: profileB, permission_mode: "guarded", unattended: false }],
    ["create_project", {
      profile_id: profileB,
      name: "B_FORBIDDEN_PROJECT_" + testRun,
      cwd: workspaceB,
    }],
    ["create_task", {
      task_id: "task-e2e-b-forbidden-" + testRun,
      project_id: projectB,
      profile_id: profileB,
      title: "B_FORBIDDEN_TASK_" + testRun,
      cwd: workspaceB,
      environment: "local",
    }],
  ]) {
    deniedScopedActions.push(await attemptDenied(action, params, bSensitive));
  }

  const inactiveAuthStatus = await attemptDenied(
    "auth_status",
    { profile_id: profileB },
    [profileB, bTitle, bSummary, workspaceB],
  );

  const activeB = await switchProfile(profileB, taskB);
  for (const paneId of unexpectedPaneIds) {
    await request("terminal_close", { pane_id: paneId }).catch(() => undefined);
  }
  if (unexpectedTaskRuntime) {
    await request("stop", { task_id: taskB }).catch(() => undefined);
  }
  const legalHistory = await request("history", { task_id: taskB });
  const legalMutation = await request("set_task", { task_id: taskB, unread: true });
  const legalRuntime = await request("runtime_snapshot", { task_id: taskB });
  const activeAuth = await request("auth_status", { profile_id: profileB });
  const invalidCancel = await attemptDenied(
    "auth_cancel",
    { attempt_id: "missing-auth-attempt-" + testRun },
    [profileB, bTitle, bSummary, workspaceB],
  );

  const terminalOpen = await request("terminal_open", {
    task_id: taskB,
    label: "E2E NativePty",
    columns: 80,
    rows: 24,
  });
  const paneId = requiredString(terminalOpen?.pane_id, "terminal pane id");
  let runningSnapshot = null;
  for (let attempt = 0; attempt < 120; attempt += 1) {
    const snapshot = await request("terminal_snapshot", { pane_id: paneId });
    if (snapshot?.is_open && snapshot?.status === "running") {
      runningSnapshot = snapshot;
      break;
    }
    if (snapshot?.status === "failed" || snapshot?.status === "exited") break;
    await delay(25);
  }
  if (!runningSnapshot) throw new Error("NativePty pane did not reach running state");
  await request("terminal_input", {
    pane_id: paneId,
    data: "printf '%s\\n' '" + ptyMarker + "'\r",
  });
  let markerObserved = false;
  let terminalRevision = 0;
  for (let attempt = 0; attempt < 160; attempt += 1) {
    const snapshot = await request("terminal_snapshot", { pane_id: paneId });
    terminalRevision = Number(snapshot?.revision ?? 0);
    if ((snapshot?.lines ?? []).some((line) => String(line).includes(ptyMarker))) {
      markerObserved = true;
      break;
    }
    await delay(25);
  }
  const resized = await request("terminal_resize", { pane_id: paneId, columns: 100, rows: 30 });
  let resizedSnapshot = null;
  for (let attempt = 0; attempt < 80; attempt += 1) {
    const snapshot = await request("terminal_snapshot", { pane_id: paneId });
    if (snapshot?.size?.columns === 100 && snapshot?.size?.rows === 30) {
      resizedSnapshot = snapshot;
      break;
    }
    await delay(25);
  }

  await switchProfile(profileA, taskA);
  const paneDenied = [];
  for (const [action, params] of [
    ["terminal_input", { pane_id: paneId, data: "echo SHOULD_NOT_RUN\r" }],
    ["terminal_snapshot", { pane_id: paneId }],
    ["terminal_resize", { pane_id: paneId, columns: 90, rows: 25 }],
    ["terminal_close", { pane_id: paneId }],
  ]) {
    paneDenied.push(await attemptDenied(action, params, [paneId, ptyMarker, bTitle, workspaceB]));
  }
  const finalA = await bootstrap();
  const finalSidebarA = await request("list_sidebar", {});
  const finalBLeaks = tokenHits(
    [finalA?.records, finalA?.sidebar, finalSidebarA?.records, finalSidebarA?.sidebar],
    bRecordTokens,
  );
  const markerLeakedOutsideTerminal = jsonContainsAny(
    [finalA?.records, finalA?.sidebar, finalSidebarA],
    [ptyMarker],
  );

  return {
    expectedRendererState: {
      profileId: profileA,
      taskId: taskA,
    },
    isolation: {
      aOwnRecordsVisible,
      bOwnRecordsVisible,
      bLeaksWhileA,
      aLeaksWhileB,
      finalBLeaks,
    },
    deniedTaskActions,
    deniedScopedActions,
    auth: {
      capability: Array.isArray(finalA?.capabilities)
        && finalA.capabilities.includes("pi_auth_control_plane"),
      inactiveStatus: inactiveAuthStatus,
      activeStatusSafe: activeAuth?.auth?.phase === "idle"
        && activeAuth?.auth?.operation === "login"
        && !jsonContainsAny(activeAuth, [sentinel, bTitle, bSummary, workspaceB]),
      invalidCancelRejected: invalidCancel.rejected,
      invalidCancelLeak: invalidCancel.leak,
    },
    legalB: {
      bootstrap: activeB?.profile?.id === profileB,
      history: legalHistory?.task_id === taskB,
      mutation: legalMutation?.task?.id === taskB && legalMutation?.task?.unread === true,
      runtimeRead: legalRuntime?.task_id === taskB,
    },
    terminal: {
      opened: terminalOpen?.task_id === taskB && terminalOpen?.status === "opening",
      markerObserved,
      revisionAdvanced: terminalRevision > 0,
      resized: resized?.accepted === true
        && resizedSnapshot?.size?.columns === 100
        && resizedSnapshot?.size?.rows === 30,
      paneDenied,
      markerLeakedOutsideTerminal,
    },
  };
})()`;

const inspectExpression = String.raw`(async () => {
  const sentinel = ${JSON.stringify(sentinel)};
  const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
  let ready = false;
  let readySamples = 0;
  for (let attempt = 0; attempt < 200; attempt += 1) {
    const shell = document.querySelector(".app-shell");
    const candidate = document.readyState === "complete"
      && Boolean(shell)
      && !shell.classList.contains("is-loading")
      && !document.querySelector(".task-data-loading")
      && typeof window.padDesktop?.bootstrap === "function";
    readySamples = candidate ? readySamples + 1 : 0;
    if (readySamples >= 3) { ready = true; break; }
    await delay(50);
  }
  if (!ready) throw new Error("renderer did not reach a ready PAD shell");

  const bootstrap = await window.padDesktop.bootstrap();
  const ping = await window.padDesktop.request("ping", {});
  const taskId = bootstrap?.records?.tasks?.[0]?.id ?? null;
  if (!taskId) throw new Error("active profile has no isolated history task");
  const history = await window.padDesktop.request("history", { task_id: taskId });
  const accountButton = document.querySelector(".account-row")
    ?? [...document.querySelectorAll("button")].find((button) => /账号|账户|account|pi/i.test(button.innerText));
  if (accountButton && accountButton.getAttribute("aria-expanded") !== "true") {
    accountButton.click();
    await delay(100);
  }

  const visible = (element) => {
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
    return { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.width, height: rect.height };
  };
  const sidebar = document.querySelector('aside[aria-label*="侧边栏"], aside.sidebar');
  const workspace = document.querySelector(".workspace");
  const shell = document.querySelector(".app-shell");
  const sidebarStyle = sidebar ? getComputedStyle(sidebar) : null;
  const attributeText = [...document.querySelectorAll("[aria-label],[placeholder],[title]")]
    .filter(visible)
    .flatMap((element) => [element.getAttribute("aria-label"), element.getAttribute("placeholder"), element.getAttribute("title")])
    .filter(Boolean);
  const surface = [document.body.innerText, ...attributeText].join("\n");
  const outerHTML = document.documentElement.outerHTML;
  const internalForbidden = /PI_CODING_AGENT_DIR|PI_SESSION_FILE|Application Support\/PAD Desktop\/v1\/profiles|\.codex|credential|token/i;
  const textareas = [...document.querySelectorAll("textarea")].filter(visible).map((element) => ({
    ariaLabel: element.getAttribute("aria-label"),
    placeholder: element.getAttribute("placeholder"),
    rect: rectValue(element),
  }));
  const alerts = [...document.querySelectorAll('[role="alert"], .error-banner')]
    .filter(visible)
    .map((element) => element.innerText.trim())
    .filter(Boolean);

  return {
    readyState: document.readyState,
    title: document.title,
    lang: document.documentElement.lang,
    innerWidth,
    innerHeight,
    outerWidth,
    outerHeight,
    bodyScrollWidth: document.body.scrollWidth,
    bodyScrollHeight: document.body.scrollHeight,
    surface,
    shellVisible: Boolean(document.querySelector(".app-shell")),
    shellProfileId: shell?.dataset.activeProfileId ?? null,
    shellSelectedTaskId: shell?.dataset.selectedTaskId ?? null,
    sidebarProfileId: sidebar?.dataset.activeProfileId ?? null,
    sidebar: sidebar ? {
      visible: visible(sidebar),
      rect: rectValue(sidebar),
      position: sidebarStyle.position,
      display: sidebarStyle.display,
      borderRadius: sidebarStyle.borderRadius,
      boxShadow: sidebarStyle.boxShadow,
    } : null,
    workspaceRect: rectValue(workspace),
    accountVisible: Boolean(accountButton && visible(accountButton)),
    accountMenuVisible: [...document.querySelectorAll('[role="menu"], .account-menu')].some(visible),
    textareas,
    alerts,
    protocolVersion: bootstrap?.protocol_version ?? null,
    pingProtocolVersion: ping?.protocol_version ?? null,
    backendStatus: bootstrap?.backend?.status ?? null,
    profileId: bootstrap?.profile?.id ?? null,
    capabilities: Array.isArray(bootstrap?.capabilities) ? bootstrap.capabilities : [],
    internalForbiddenPresent: internalForbidden.test(surface + "\\n" + outerHTML),
    sentinelLeaks: {
      dom: surface.includes(sentinel) || outerHTML.includes(sentinel),
      bootstrap: JSON.stringify(bootstrap).includes(sentinel),
      history: JSON.stringify(history).includes(sentinel),
    },
    historyChecked: true,
  };
})()`;

async function screenshot(name) {
  const result = await call("Page.captureScreenshot", {
    format: "png",
    fromSurface: true,
    captureBeyondViewport: false,
  });
  await Bun.write(`${artifactDir}/${name}.png`, Buffer.from(result.data, "base64"));
}

async function saveDom(name) {
  const html = await evaluate("document.documentElement.outerHTML");
  await Bun.write(`${artifactDir}/${name}.html`, html);
}

socket.addEventListener("open", async () => {
  try {
    await call("Page.enable");
    await call("Runtime.enable");
    await call("Accessibility.enable");
    await call("Page.bringToFront");
    const workflow = await evaluate(workflowExpression);
    await call("Page.reload", { ignoreCache: true });
    const wide = await evaluateAfterReload(inspectExpression);
    const axTree = await call("Accessibility.getFullAXTree");
    await Bun.write(`${artifactDir}/accessibility.json`, JSON.stringify(axTree, null, 2));
    const interactiveRoles = new Set(["button", "textbox", "checkbox", "tab"]);
    const interactive = axTree.nodes
      .filter((node) => !node.ignored && interactiveRoles.has(String(node.role?.value ?? "").toLowerCase()))
      .filter((node) => node.properties?.some((property) => property.name === "focusable" && property.value?.value === true))
      .map((node) => ({
        role: String(node.role?.value ?? ""),
        name: String(node.name?.value ?? "").trim(),
      }));
    const axText = interactive.map((node) => `${node.role}:${node.name}`).join("\n");
    const axForbidden = /\b(?:preview|mock)\b|PI_CODING_AGENT_DIR|PI_SESSION_FILE|Application Support\/PAD Desktop\/v1\/profiles|\.codex|credential|token/i;
    const accessibility = {
      interactiveCount: interactive.length,
      unnamedOrNonChinese: interactive.filter((node) => !node.name || !/[\u3400-\u9fff]/.test(node.name)),
      forbiddenPresent: axForbidden.test(axText),
      sentinelLeak: axText.includes(sentinel),
    };
    await saveDom("wide-dom");
    await screenshot("wide-window");

    await evaluate("window.resizeTo(480, 600); true");
    await delay(250);
    await call("Page.reload", { ignoreCache: true });
    const narrow = await evaluateAfterReload(inspectExpression);
    await saveDom("narrow-dom");
    await screenshot("narrow-window");
    process.stdout.write(JSON.stringify({
      workflow,
      wide,
      narrow,
      accessibility,
      narrowBounds: { width: narrow.outerWidth, height: narrow.outerHeight },
    }));
    socket.close();
  } catch (error) {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exitCode = 1;
    socket.close();
  }
});

setTimeout(() => {
  console.error("CDP capture timed out");
  process.exit(1);
}, 60000).unref();
"""


CDP_CLOSE_SCRIPT = r"""
const url = process.env.PAD_CDP_WS;
if (!url) throw new Error("missing PAD_CDP_WS");
const socket = new WebSocket(url);
socket.addEventListener("open", () => socket.send(JSON.stringify({ id: 1, method: "Browser.close" })));
socket.addEventListener("message", () => { socket.close(); process.exit(0); });
setTimeout(() => process.exit(0), 1500);
"""


def run_cdp_capture(
    bundle: Bundle,
    target: dict[str, Any],
    artifact_dir: pathlib.Path,
    sentinel: str,
    workspace: pathlib.Path,
) -> dict[str, Any]:
    websocket_url = target.get("webSocketDebuggerUrl")
    require(isinstance(websocket_url, str), "CDP target has no WebSocket URL")
    test_run = f"{int(time.time())}-{secrets.token_hex(8)}"
    pty_marker = f"PAD_NativePty_终端_{secrets.token_hex(24)}"
    env = inherited_environment_allowlist()
    env.update(
        {
            "PAD_CDP_WS": websocket_url,
            "PAD_CDP_ARTIFACT_DIR": str(artifact_dir),
            "PAD_E2E_SENTINEL": sentinel,
            "PAD_E2E_WORKSPACE": str(workspace),
            "PAD_E2E_TEST_RUN": test_run,
            "PAD_E2E_PTY_MARKER": pty_marker,
        }
    )
    result = run_command([str(bundle.bun), "-e", CDP_CAPTURE_SCRIPT], env=env, timeout=75)
    try:
        captured = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise AcceptanceError(f"CDP capture did not return JSON: {result.stdout[:500]}") from error
    require(isinstance(captured, dict), "CDP capture result is not an object")
    return captured


def close_over_cdp(bundle: Bundle, target: dict[str, Any]) -> None:
    websocket_url = target.get("webSocketDebuggerUrl")
    if not isinstance(websocket_url, str):
        return
    env = inherited_environment_allowlist()
    env["PAD_CDP_WS"] = websocket_url
    try:
        run_command([str(bundle.bun), "-e", CDP_CLOSE_SCRIPT], env=env, timeout=3)
    except AcceptanceError:
        pass


EXPECTED_CROSS_PROFILE_TASK_DENIALS = {
    "history",
    "start_task",
    "prompt",
    "poll",
    "abort",
    "stop",
    "stop_task",
    "retry_task",
    "get_messages",
    "get_state",
    "get_entries",
    "set_task",
    "set_model",
    "set_thinking_level",
    "respond_ui",
    "extension_ui_response",
    "runtime_snapshot",
    "terminal_open",
}

EXPECTED_CROSS_PROFILE_PANE_DENIALS = {
    "terminal_input",
    "terminal_snapshot",
    "terminal_resize",
    "terminal_close",
}

EXPECTED_CROSS_PROFILE_SCOPED_DENIALS = {
    "provider_status",
    "set_profile",
    "create_project",
    "create_task",
}


def assert_boundary_denials(
    values: Any,
    expected_actions: set[str],
    label: str,
) -> None:
    require(isinstance(values, list), f"{label} denial evidence is missing")
    by_action = {
        str(item.get("action")): item
        for item in values
        if isinstance(item, dict) and isinstance(item.get("action"), str)
    }
    require(set(by_action) == expected_actions, f"{label} coverage is incomplete: {sorted(by_action)}")
    unexpected_success = [
        action for action, item in by_action.items() if item.get("rejected") is not True
    ]
    require(not unexpected_success, f"{label} unexpectedly succeeded: {sorted(unexpected_success)}")
    wrong_reason = [
        action for action, item in by_action.items() if item.get("boundary") is not True
    ]
    require(
        not wrong_reason,
        f"{label} did not fail at the active-profile boundary: {sorted(wrong_reason)}",
    )
    disclosures = [action for action, item in by_action.items() if item.get("leak") is True]
    require(not disclosures, f"{label} errors disclosed private content or paths: {sorted(disclosures)}")


def assert_security_workflow(capture: dict[str, Any]) -> None:
    workflow = capture.get("workflow")
    require(isinstance(workflow, dict), "dual-profile security workflow evidence is missing")

    isolation = workflow.get("isolation")
    require(isinstance(isolation, dict), "profile-record isolation evidence is missing")
    require(isolation.get("aOwnRecordsVisible") is True, "Profile A cannot see its own records")
    require(isolation.get("bOwnRecordsVisible") is True, "Profile B cannot see its own records")
    for key, description in (
        ("bLeaksWhileA", "Profile A bootstrap/sidebar exposed Profile B task or project data"),
        ("aLeaksWhileB", "Profile B bootstrap/sidebar exposed Profile A task or project data"),
        ("finalBLeaks", "Profile B data remained visible after switching back to Profile A"),
    ):
        require(isolation.get(key) == 0, description)

    assert_boundary_denials(
        workflow.get("deniedTaskActions"),
        EXPECTED_CROSS_PROFILE_TASK_DENIALS,
        "cross-profile task action",
    )
    assert_boundary_denials(
        workflow.get("deniedScopedActions"),
        EXPECTED_CROSS_PROFILE_SCOPED_DENIALS,
        "cross-profile profile/project action",
    )

    legal = workflow.get("legalB")
    require(isinstance(legal, dict), "active Profile B legal-access evidence is missing")
    failed_legal = [name for name, passed in legal.items() if passed is not True]
    require(not failed_legal, f"active Profile B legal access failed: {sorted(failed_legal)}")

    auth = workflow.get("auth")
    require(isinstance(auth, dict), "authentication control-plane evidence is missing")
    require(auth.get("capability") is True, "Pi authentication capability is not advertised")
    inactive_status = auth.get("inactiveStatus")
    require(isinstance(inactive_status, dict), "inactive-profile auth_status evidence is missing")
    require(inactive_status.get("rejected") is True, "inactive-profile auth_status was accepted")
    require(inactive_status.get("boundary") is True, "auth_status missed the profile boundary")
    require(inactive_status.get("leak") is False, "auth_status error disclosed private data")
    require(auth.get("activeStatusSafe") is True, "active auth_status was not a safe idle snapshot")
    require(auth.get("invalidCancelRejected") is True, "unknown auth_cancel attempt was accepted")
    require(auth.get("invalidCancelLeak") is False, "auth_cancel error disclosed private data")

    terminal = workflow.get("terminal")
    require(isinstance(terminal, dict), "NativePty workflow evidence is missing")
    for key, description in (
        ("opened", "NativePty did not open for the active Profile B task"),
        ("markerObserved", "random Unicode marker was not observed in the NativePty snapshot"),
        ("revisionAdvanced", "NativePty revision did not advance"),
        ("resized", "NativePty resize did not reach 100x30"),
    ):
        require(terminal.get(key) is True, description)
    require(
        terminal.get("markerLeakedOutsideTerminal") is False,
        "NativePty marker leaked into task/sidebar/bootstrap data",
    )
    assert_boundary_denials(
        terminal.get("paneDenied"),
        EXPECTED_CROSS_PROFILE_PANE_DENIALS,
        "cross-profile terminal pane action",
    )


def assert_ui_capture(capture: dict[str, Any], expected_protocol_minimum: int) -> None:
    require(
        expected_protocol_minimum >= EXPECTED_PROTOCOL_MINIMUM,
        f"protocol minimum cannot weaken the required v{EXPECTED_PROTOCOL_MINIMUM} gate",
    )
    assert_security_workflow(capture)
    wide = capture.get("wide")
    narrow = capture.get("narrow")
    require(isinstance(wide, dict) and isinstance(narrow, dict), "missing wide/narrow UI capture")
    workflow = capture.get("workflow")
    expected_renderer = workflow.get("expectedRendererState") if isinstance(workflow, dict) else None
    require(isinstance(expected_renderer, dict), "expected renderer state evidence is missing")
    for phase, value in (("wide", wide), ("narrow", narrow)):
        require(
            value.get("shellProfileId") == expected_renderer.get("profileId"),
            f"{phase} renderer shell did not load the active Profile A state",
        )
        require(
            value.get("sidebarProfileId") == expected_renderer.get("profileId"),
            f"{phase} sidebar profile differs from the renderer shell",
        )
        require(
            value.get("shellSelectedTaskId") == expected_renderer.get("taskId"),
            f"{phase} renderer did not restore the selected Profile A task",
        )

    require(wide.get("title") == EXPECTED_PRODUCT_NAME, f"unexpected document title: {wide.get('title')}")
    require(str(wide.get("lang", "")).lower().startswith("zh"), "renderer language is not Chinese")
    require(wide.get("shellVisible") is True, "Chinese application shell is missing")
    require(not wide.get("alerts"), f"renderer exposed an error alert: {wide.get('alerts')}")

    surface = str(wide.get("surface", ""))
    chinese_count = len(re.findall(r"[\u3400-\u9fff]", surface))
    require(chinese_count >= 20, f"renderer is not fully localized ({chinese_count} Chinese characters)")
    for alternatives, label in (
        (("新任务", "新建任务"), "new task"),
        (("项目",), "projects"),
        (("搜索",), "search"),
        (("设置",), "settings"),
        (("任务输入", "向 Pi 描述一个任务"), "composer"),
    ):
        require(any(text in surface for text in alternatives), f"Chinese {label} label is missing")
    require(wide.get("accountVisible") is True, "account switcher is not visible")
    require(wide.get("accountMenuVisible") is True, "account switcher does not expose its account list")
    require("切换账号" in surface or "切换账户" in surface, "account switching is not localized")
    require(bool(wide.get("textareas")), "visible task composer textarea is missing")

    accessibility = capture.get("accessibility")
    require(isinstance(accessibility, dict), "CDP accessibility snapshot is missing")
    require(int(accessibility.get("interactiveCount", 0)) > 0, "AX tree has no focusable controls")
    require(
        not accessibility.get("unnamedOrNonChinese"),
        f"focusable controls lack Chinese AX names: {accessibility.get('unnamedOrNonChinese')}",
    )
    require(accessibility.get("forbiddenPresent") is False, "AX tree exposes preview/mock/internal Pi data")
    require(accessibility.get("sentinelLeak") is False, "sensitive env sentinel leaked into AX tree")

    forbidden = re.compile(
        r"\b(?:preview|mock)\b|预览|模拟界面|演示版|PI_CODING_AGENT_DIR|PI_SESSION_FILE|"
        r"Application Support/PAD Desktop/v1/profiles|\.codex|credential|token",
        re.IGNORECASE,
    )
    match = forbidden.search(surface)
    require(match is None, f"renderer still exposes preview/mock wording: {match.group(0) if match else ''}")
    require(wide.get("internalForbiddenPresent") is False, "DOM exposes internal path/credential wording")
    require(wide.get("historyChecked") is True, "isolated task history was not inspected")
    for phase, value in (("wide", wide), ("narrow", narrow)):
        leaks = value.get("sentinelLeaks")
        require(isinstance(leaks, dict), f"{phase} sentinel result is missing")
        leaked_surfaces = [name for name, leaked in leaks.items() if leaked]
        require(not leaked_surfaces, f"sensitive env sentinel leaked into {phase}: {leaked_surfaces}")

    sidebar = wide.get("sidebar")
    workspace = wide.get("workspaceRect")
    require(isinstance(sidebar, dict) and sidebar.get("visible") is True, "sidebar is not visible")
    require(isinstance(workspace, dict), "workspace geometry is missing")
    sidebar_rect = sidebar.get("rect")
    require(isinstance(sidebar_rect, dict), "sidebar geometry is missing")
    require(sidebar.get("position") not in ("fixed", "absolute"), "wide sidebar is floating")
    require(sidebar.get("boxShadow") in ("none", ""), "wide sidebar still has a floating shadow")
    require(sidebar.get("borderRadius") in ("0px", "0px 0px 0px 0px"), "wide sidebar is rounded/floating")
    gap = float(workspace["left"]) - float(sidebar_rect["right"])
    require(abs(gap) <= 2.0, f"sidebar is not tiled against workspace (gap={gap:.1f}px)")

    protocol = wide.get("protocolVersion")
    ping_protocol = wide.get("pingProtocolVersion")
    require(isinstance(protocol, int), f"bootstrap has no protocol version: {protocol!r}")
    require(protocol >= expected_protocol_minimum, f"protocol v{protocol} is older than v{expected_protocol_minimum}")
    require(ping_protocol == protocol, f"ping/bootstrap protocol mismatch: {ping_protocol} vs {protocol}")
    require(wide.get("backendStatus") == "ready", "Rust backend is not ready")

    bounds = capture.get("narrowBounds")
    require(isinstance(bounds, dict), "narrow macOS window bounds are missing")
    require(int(bounds.get("width", 0)) == 480, f"narrow window width is {bounds.get('width')}")
    require(int(bounds.get("height", 0)) == 600, f"narrow window height is {bounds.get('height')}")
    require(narrow.get("shellVisible") is True, "renderer crashed after a 480x600 startup reload")
    require(not narrow.get("alerts"), f"narrow renderer exposed an error: {narrow.get('alerts')}")
    require(bool(narrow.get("textareas")), "composer disappeared at 480x600")
    require(420 <= int(narrow.get("innerWidth", 0)) <= 500, "narrow renderer width is invalid")
    require(500 <= int(narrow.get("innerHeight", 0)) <= 620, "narrow renderer height is invalid")


def wait_for_sidecar(bundle: Bundle, main_pid: int, timeout: float = 10.0) -> int:
    deadline = time.monotonic() + timeout
    expected_pad = str(bundle.pad)
    while time.monotonic() < deadline:
        table = process_table()
        family = descendants(main_pid, table)
        for pid in family:
            command = table.get(pid, (0, ""))[1]
            if expected_pad in command and "__internal" in command and "desktop-server" in command:
                return pid
        time.sleep(0.1)
    raise AcceptanceError("packaged Rust desktop-server sidecar was not observed")


def wait_for_sqlite(data_root: pathlib.Path, timeout: float = 10.0) -> pathlib.Path:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        candidates = sorted(data_root.rglob("*.sqlite")) if data_root.exists() else []
        if candidates:
            database = candidates[0]
            require(database.resolve().is_relative_to(data_root.resolve()), "SQLite escaped isolated data root")
            with sqlite3.connect(f"file:{database}?mode=ro", uri=True) as connection:
                integrity = connection.execute("PRAGMA quick_check").fetchone()
                tables = connection.execute(
                    "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
                ).fetchall()
            require(integrity == ("ok",), f"SQLite quick_check failed: {integrity}")
            require(bool(tables), "isolated SQLite has no schema")
            if os.name == "posix":
                require(stat.S_IMODE(database.stat().st_mode) == 0o600, "SQLite mode is not 0600")
            return database
        time.sleep(0.1)
    raise AcceptanceError(f"no isolated SQLite appeared under {data_root}")


def wait_for_exit(process: subprocess.Popen[bytes], timeout: float) -> bool:
    try:
        process.wait(timeout=timeout)
        return True
    except subprocess.TimeoutExpired:
        return False


def graceful_quit(bundle: Bundle, process: subprocess.Popen[bytes], target: dict[str, Any]) -> str:
    close_over_cdp(bundle, target)
    if wait_for_exit(process, 6):
        return "CDP Browser.close"

    # An older Swift fallback may share this bundle identifier.  Only use a
    # bundle-id AppleEvent when this Electron process is the sole matching app;
    # otherwise the event could close a user-owned instance.
    same_bundle_executables = matching_processes(
        [f"/{EXPECTED_PRODUCT_NAME}.app/Contents/MacOS/PADDesktop"]
    )
    other_apps = [(pid, command) for pid, command in same_bundle_executables if pid != process.pid]
    require(not other_apps, f"refusing ambiguous bundle-id quit; other apps: {other_apps}")
    applescript = f'tell application id "{bundle.bundle_id}" to quit'
    try:
        run_command(["/usr/bin/osascript", "-e", applescript], timeout=5)
    except AcceptanceError:
        pass
    if wait_for_exit(process, 6):
        return "AppleEvent"
    raise AcceptanceError("PAD Desktop did not exit through CDP Browser.close or AppleEvent")


def process_group_alive(process_group_id: int) -> bool:
    try:
        os.killpg(process_group_id, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def wait_for_process_group_exit(
    process: subprocess.Popen[bytes], process_group_id: int, timeout: float
) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        process.poll()
        if not process_group_alive(process_group_id):
            return True
        time.sleep(0.05)
    process.poll()
    return not process_group_alive(process_group_id)


def cleanup_process_group(process: subprocess.Popen[bytes]) -> None:
    # start_new_session=True makes the launch PID the process-group ID.  The
    # group can outlive its Electron leader, so never skip cleanup merely
    # because Popen.poll() reports that the leader exited first.
    process_group_id = process.pid
    if not process_group_alive(process_group_id):
        process.poll()
        return
    try:
        os.killpg(process_group_id, signal.SIGTERM)
    except ProcessLookupError:
        return
    if wait_for_process_group_exit(process, process_group_id, 3):
        return
    try:
        os.killpg(process_group_id, signal.SIGKILL)
    except ProcessLookupError:
        return
    require(
        wait_for_process_group_exit(process, process_group_id, 2),
        f"PAD Desktop process group {process_group_id} survived forced cleanup",
    )


def tail(path: pathlib.Path, maximum_bytes: int = 8_000) -> str:
    try:
        with path.open("rb") as handle:
            handle.seek(0, os.SEEK_END)
            size = handle.tell()
            handle.seek(max(0, size - maximum_bytes))
            return handle.read().decode("utf-8", errors="replace")
    except OSError:
        return ""


def check_running_app(
    bundle: Bundle,
    scratch: pathlib.Path,
    artifact_dir: pathlib.Path,
    expected_protocol_minimum: int,
) -> None:
    existing = matching_processes([str(bundle.executable), str(bundle.pad)])
    require(not existing, f"another PAD Desktop instance is running: {existing}")

    isolated_home = scratch / "app-home"
    data_root = scratch / "desktop-data"
    user_data = scratch / "electron-user-data"
    workspace = scratch / "workspace"
    for directory in (isolated_home, data_root, user_data, workspace):
        directory.mkdir(mode=0o700)
    for directory in (workspace / "profile-a", workspace / "profile-b"):
        directory.mkdir(mode=0o700)
    port = free_tcp_port()
    stdout_log = artifact_dir / "app-stdout.log"
    stderr_log = artifact_dir / "app-stderr.log"
    env = safe_environment(isolated_home, data_root)
    env["ELECTRON_ENABLE_LOGGING"] = "1"
    # Exercise Electron's protection-only CODEX_HOME mapping with a controlled
    # path outside the isolated HOME. The real launching value is never
    # inherited, opened, hashed, or written by this acceptance test.
    custom_codex_home = scratch / "external-custom-codex-home"
    env["CODEX_HOME"] = str(custom_codex_home)
    sentinel = f"PAD_E2E_SENSITIVE_{secrets.token_hex(24)}"
    env["PAD_E2E_SENSITIVE_SENTINEL"] = sentinel
    protected = protected_locations(isolated_home, custom_codex_home)
    before = create_protected_sentinels(isolated_home, sentinel, custom_codex_home)
    assert_complete_protected_snapshot(before, "before-launch")
    assert_protected_sentinel_contents(isolated_home, sentinel, custom_codex_home)
    (artifact_dir / "protected-before.json").write_text(
        json.dumps(before, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    assert_protected_data_roots_fail_closed(
        bundle, isolated_home, custom_codex_home, sentinel, artifact_dir
    )

    with stdout_log.open("wb") as stdout, stderr_log.open("wb") as stderr:
        process = subprocess.Popen(
            [
                str(bundle.executable),
                f"--remote-debugging-port={port}",
                "--remote-allow-origins=*",
                f"--user-data-dir={user_data}",
                "--no-first-run",
                "--disable-default-apps",
                "--window-size=1280,820",
            ],
            env=env,
            stdout=stdout,
            stderr=stderr,
            start_new_session=True,
        )

    target: dict[str, Any] | None = None
    try:
        target = wait_for_cdp(port, process)
        sidecar_pid = wait_for_sidecar(bundle, process.pid)
        capture = run_cdp_capture(bundle, target, artifact_dir, sentinel, workspace)
        (artifact_dir / "ui-capture.json").write_text(
            json.dumps(capture, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        assert_ui_capture(capture, expected_protocol_minimum)
        database = wait_for_sqlite(data_root)
        table_before_quit = process_table()
        family = descendants(process.pid, table_before_quit)
        sidecar_family = descendants(sidecar_pid, table_before_quit)
        pty_process_ids = sidecar_family - {sidecar_pid}
        require(pty_process_ids, "real NativePty child process was not observed before quit")
        assert_processes_hold_no_protected_descriptors(family, protected)

        quit_method = graceful_quit(bundle, process, target)
        require(process.returncode == 0, f"PAD Desktop exited with code {process.returncode}")
        deadline = time.monotonic() + 5
        leftovers: list[tuple[int, str]] = []
        descendant_survivors: list[int] = []
        process_group_survived = True
        while time.monotonic() < deadline:
            table_after_quit = process_table()
            leftovers = [
                (pid, command)
                for pid, (_parent, command) in table_after_quit.items()
                if is_test_bundle_process(command, bundle.app)
            ]
            descendant_survivors = sorted(pid for pid in family if pid in table_after_quit)
            process_group_survived = process_group_alive(process.pid)
            if not leftovers and not descendant_survivors and not process_group_survived:
                break
            time.sleep(0.1)
        require(not leftovers, f"packaged app left processes behind: {leftovers}")
        require(
            not descendant_survivors,
            f"Electron/Rust/NativePty descendants survived app quit: {descendant_survivors}",
        )
        require(
            not process_group_survived,
            f"PAD Desktop launch process group {process.pid} survived app quit",
        )

        after = snapshot_protected_metadata(protected)
        assert_complete_protected_snapshot(after, "after-quit")
        assert_protected_sentinel_contents(isolated_home, sentinel, custom_codex_home)
        (artifact_dir / "protected-after.json").write_text(
            json.dumps(after, ensure_ascii=False, indent=2), encoding="utf-8"
        )
        changed = [label for label in before if before[label] != after[label]]
        require(not changed, f"protected sentinel metadata changed: {changed}")
        print(
            f"[PASS] real app PID {process.pid}, Rust sidecar PID {sidecar_pid}, "
            f"NativePty children {len(pty_process_ids)}, protocol v{capture['wide']['protocolVersion']}, "
            f"SQLite {database}, quit via {quit_method}"
        )
        print("[PASS] dual-Profile records, task actions, auth state, and terminal panes are isolated")
        print("[PASS] Chinese shell, tiled sidebar, account switcher, composer, 480x600 reload")
        print("[PASS] protected sentinel trees were unchanged and no live descriptor pointed into them")
        print("[PASS] user's real Codex/ChatGPT data was never traversed by the acceptance test")
    except Exception as error:
        diagnostics = tail(stderr_log)
        if diagnostics:
            print(f"--- PAD Desktop stderr ---\n{diagnostics}", file=sys.stderr)
        raise error
    finally:
        # Also run after a graceful leader exit: Electron/Rust/PTY descendants
        # may still retain the launch process group even when Popen has exited.
        cleanup_process_group(process)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", type=pathlib.Path, default=DEFAULT_APP)
    parser.add_argument(
        "--artifact-dir",
        type=pathlib.Path,
        help="directory for DOM, screenshots, logs, and metadata summaries",
    )
    parser.add_argument(
        "--protocol-minimum",
        type=int,
        default=EXPECTED_PROTOCOL_MINIMUM,
        help="minimum Desktop protocol accepted by the packaged renderer",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    scratch = pathlib.Path(tempfile.mkdtemp(prefix="pad-desktop-electron-e2e-state-"))
    artifact_dir = (
        args.artifact_dir.expanduser().resolve()
        if args.artifact_dir
        else pathlib.Path(tempfile.mkdtemp(prefix="pad-desktop-electron-e2e-artifacts-"))
    )
    artifact_dir.mkdir(parents=True, exist_ok=True)
    try:
        bundle = check_bundle(args.app, scratch)
        check_running_app(bundle, scratch, artifact_dir, args.protocol_minimum)
        print(f"[PASS] artifacts: {artifact_dir}")
        return 0
    except (AcceptanceError, AssertionError, OSError, sqlite3.Error) as error:
        print(f"[FAIL] {error}", file=sys.stderr)
        print(f"[INFO] artifacts retained at: {artifact_dir}", file=sys.stderr)
        return 1
    finally:
        # DOM/screenshots/logs survive for diagnosis; isolated app state does not.
        shutil.rmtree(scratch, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
