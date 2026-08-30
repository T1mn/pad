#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="${0:A:h}"
APP_DIR="${SCRIPT_DIR:h}"
TARGET_APP="/Applications/PAD Desktop.app"
SOURCE_APP="${APP_DIR}/out/PAD Desktop-darwin-arm64/PAD Desktop.app"
HEALTH_APP="${TARGET_APP}"
CHECK_ONLY=0
LAUNCH_AFTER_INSTALL=0
INSTALL_STAGE=""
BACKUP_APP=""
FAILED_NEW_APP=""
INSTALL_STARTED=0
INSTALL_COMPLETE=0
HEALTH_PID=""
HEALTH_PROCESS_GROUP=""
HEALTH_COMPLETE=0
EXPECTED_PI_VERSION="0.84.4"
EXPECTED_BUN_VERSION="1.3.14"
MINIMUM_MACOS_VERSION="13.0"

usage() {
  cat <<'EOF'
Usage: install-electron-app.sh [--source /path/to/PAD Desktop.app] [--check-only] [--launch]

The destination is fixed to /Applications/PAD Desktop.app and cannot be overridden.
An existing app is moved to a timestamped, recoverable backup before installation.
`--check-only` performs the same isolated health probe without writing to /Applications.
The new app is committed only after that renderer + protocol-v2 probe exits cleanly.
EOF
}

fail() {
  echo "PAD Desktop install error: $*" >&2
  exit 1
}

cleanup_install() {
  local exit_code=$?
  trap - EXIT
  set +e
  cleanup_health_process
  if (( exit_code != 0 && INSTALL_STARTED && ! INSTALL_COMPLETE )); then
    if [[ -d "${TARGET_APP}" && ! -L "${TARGET_APP}" && -n "${FAILED_NEW_APP}" && ! -e "${FAILED_NEW_APP}" ]]; then
      /bin/mv "${TARGET_APP}" "${FAILED_NEW_APP}"
      echo "Preserved failed new bundle at: ${FAILED_NEW_APP}" >&2
    fi
    if [[ -n "${BACKUP_APP}" && -d "${BACKUP_APP}" && ! -e "${TARGET_APP}" ]]; then
      /usr/bin/ditto "${BACKUP_APP}" "${TARGET_APP}"
      echo "Restored previous PAD Desktop from: ${BACKUP_APP}" >&2
    fi
  fi
  if [[ -n "${INSTALL_STAGE}" && -d "${INSTALL_STAGE}" ]]; then
    case "${INSTALL_STAGE}" in
      /Applications/.pad-desktop-install.*|/tmp/pad-desktop-install-check.*)
        rm -rf -- "${INSTALL_STAGE}"
        ;;
    esac
  fi
  exit "${exit_code}"
}
trap cleanup_install EXIT

while (( $# )); do
  case "$1" in
    --source)
      (( $# >= 2 )) || fail "--source requires an app bundle path"
      SOURCE_APP="$2"
      shift 2
      ;;
    --check-only)
      CHECK_ONLY=1
      shift
      ;;
    --launch)
      LAUNCH_AFTER_INSTALL=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

SOURCE_APP="$(/usr/bin/python3 -c 'import os, sys; print(os.path.abspath(sys.argv[1]))' "${SOURCE_APP}")"
[[ "${SOURCE_APP}" != "${TARGET_APP}" ]] || fail "source and destination must be different"

assert_arm64_only() {
  local binary="$1"
  local architectures
  [[ -x "${binary}" ]] || fail "required executable is missing: ${binary}"
  architectures="$(/usr/bin/lipo -archs "${binary}" 2>/dev/null)" || fail "not a Mach-O executable: ${binary}"
  [[ "${architectures}" == "arm64" ]] || fail "expected arm64-only executable, got '${architectures}': ${binary}"
}

extract_semver() {
  /usr/bin/python3 - "$1" <<'PY'
import re
import sys

match = re.search(r"(?<!\d)(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)", sys.argv[1])
if match is None:
    raise SystemExit(1)
print(match.group(1))
PY
}

assert_bundle_minos_at_most() {
  /usr/bin/python3 - "$1" "$2" <<'PY'
import pathlib
import re
import stat
import subprocess
import sys

bundle = pathlib.Path(sys.argv[1])
maximum_text = sys.argv[2]

def version(value: str) -> tuple[int, ...]:
    return tuple(int(part) for part in value.split("."))

maximum = version(maximum_text)
required = {
    "Contents/MacOS/PADDesktop",
    "Contents/Resources/bin/bun",
    "Contents/Frameworks/Electron Framework.framework/Versions/A/Electron Framework",
    "Contents/Frameworks/PAD Desktop Helper.app/Contents/MacOS/PAD Desktop Helper",
    "Contents/Frameworks/PAD Desktop Helper (GPU).app/Contents/MacOS/PAD Desktop Helper (GPU)",
    "Contents/Frameworks/PAD Desktop Helper (Renderer).app/Contents/MacOS/PAD Desktop Helper (Renderer)",
    "Contents/Frameworks/PAD Desktop Helper (Plugin).app/Contents/MacOS/PAD Desktop Helper (Plugin)",
}
seen: set[str] = set()
checked = 0
for candidate in sorted(bundle.rglob("*")):
    try:
        metadata = candidate.stat()
    except OSError:
        continue
    if not stat.S_ISREG(metadata.st_mode) or not metadata.st_mode & 0o111:
        continue
    kind = subprocess.run(
        ["/usr/bin/file", "-b", str(candidate)], check=True, text=True, stdout=subprocess.PIPE
    ).stdout
    if not kind.startswith("Mach-O"):
        continue
    loads = subprocess.run(
        ["/usr/bin/otool", "-m", "-l", str(candidate)], check=True, text=True, stdout=subprocess.PIPE
    ).stdout.splitlines()
    minimums: list[str] = []
    want_version = False
    for line in loads:
        stripped = line.strip()
        if stripped in {"cmd LC_BUILD_VERSION", "cmd LC_VERSION_MIN_MACOSX"}:
            want_version = True
            continue
        if want_version:
            match = re.fullmatch(r"(?:minos|version)\s+([0-9]+(?:\.[0-9]+)*)", stripped)
            if match:
                minimums.append(match.group(1))
                want_version = False
    relative = candidate.relative_to(bundle).as_posix()
    if relative in required:
        seen.add(relative)
    if not minimums:
        raise SystemExit(f"Mach-O has no macOS deployment target: {relative}")
    for minimum in minimums:
        if version(minimum) > maximum:
            raise SystemExit(f"{relative} requires macOS {minimum}, above {maximum_text}")
        checked += 1
missing = sorted(required - seen)
if missing:
    raise SystemExit(f"critical Mach-O files were not checked: {', '.join(missing)}")
if checked < len(required):
    raise SystemExit("too few Mach-O deployment targets were checked")
PY
}

assert_internal_symlinks() {
  /usr/bin/python3 - "$1" <<'PY'
import os
import sys

root = os.path.realpath(sys.argv[1])
for directory, directories, files in os.walk(root, followlinks=False):
    for name in directories + files:
        candidate = os.path.join(directory, name)
        if not os.path.islink(candidate):
            continue
        link = os.readlink(candidate)
        if os.path.isabs(link):
            raise SystemExit(f"absolute symlink is not allowed: {candidate} -> {link}")
        if os.path.commonpath((root, os.path.realpath(candidate))) != root:
            raise SystemExit(f"escaping symlink is not allowed: {candidate} -> {link}")
PY
}

plist_value() {
  /usr/libexec/PlistBuddy -c "Print :$2" "$1" 2>/dev/null
}

verify_runtime_evidence() {
  local resources="$1"
  (
    cd "${resources}"
    /usr/bin/shasum -a 256 -c release-evidence/runtime-SHA256SUMS.txt >/dev/null
  ) || fail "runtime checksum evidence does not match the app bundle"
  /usr/bin/python3 - \
    "${resources}/release-evidence/runtime-manifest.json" \
    "${resources}/release-evidence/runtime-sbom.spdx.json" \
    "${EXPECTED_PI_VERSION}" \
    "${EXPECTED_BUN_VERSION}" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
sbom = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
pi_version, bun_version = sys.argv[3:]
components = {item.get("name"): item.get("version") for item in manifest.get("components", [])}
packages = {item.get("name"): item.get("versionInfo") for item in sbom.get("packages", [])}
if manifest.get("schema") != "cn.ghostcloud.pad.desktop.runtime-evidence.v1":
    raise SystemExit("invalid runtime evidence schema")
if manifest.get("target") != "darwin-arm64" or manifest.get("minimum_macos") != "13.0":
    raise SystemExit("runtime evidence target does not match the install target")
if components.get("Pi coding agent") != pi_version or components.get("Bun") != bun_version:
    raise SystemExit("runtime evidence does not contain the pinned Pi/Bun versions")
if packages.get("@earendil-works/pi-coding-agent") != pi_version or packages.get("Bun") != bun_version:
    raise SystemExit("runtime SPDX SBOM does not contain the pinned Pi/Bun versions")
PY
}

verify_bundle() {
  local app_bundle="$1"
  local info="${app_bundle}/Contents/Info.plist"
  local resources="${app_bundle}/Contents/Resources"
  local entitlements
  local pi_version
  local bun_version
  local signature_details

  [[ -d "${app_bundle}" && ! -L "${app_bundle}" ]] || fail "app bundle is missing or is a symlink: ${app_bundle}"
  /usr/bin/plutil -lint "${info}" >/dev/null
  [[ "$(plist_value "${info}" CFBundleIdentifier)" == cn.ghostcloud.pad.desktop ]] || fail "unexpected bundle identifier"
  [[ "$(plist_value "${info}" CFBundleExecutable)" == PADDesktop ]] || fail "unexpected bundle executable"
  [[ "$(plist_value "${info}" CFBundleDisplayName)" == 'PAD Desktop' ]] || fail "unexpected bundle display name"
  [[ "$(plist_value "${info}" CFBundleName)" == 'PAD Desktop' ]] || fail "unexpected bundle name"
  [[ "$(plist_value "${info}" LSMinimumSystemVersion)" == 13.0 ]] || fail "unexpected minimum macOS version"
  [[ "$(plist_value "${info}" CFBundleShortVersionString)" == <->.<->.<->* ]] || fail "invalid bundle version"

  for required in \
    "${resources}/app.asar" \
    "${resources}/bin/bun" \
    "${resources}/bin/node" \
    "${resources}/bin/pi" \
    "${resources}/pi/package.json" \
    "${resources}/pi/dist/bun/cli.js" \
    "${resources}/pi/dist/bundle/cli.js" \
    "${resources}/release-evidence/runtime-manifest.json" \
    "${resources}/release-evidence/runtime-sbom.spdx.json" \
    "${resources}/release-evidence/runtime-SHA256SUMS.txt"; do
    [[ -f "${required}" ]] || fail "bundle resource is missing: ${required}"
  done
  for executable in \
    "${app_bundle}/Contents/MacOS/PADDesktop" \
    "${resources}/bin/bun" \
    "${resources}/bin/node" \
    "${resources}/bin/pi"; do
    [[ -x "${executable}" ]] || fail "bundle executable bit is missing: ${executable}"
  done

  assert_arm64_only "${app_bundle}/Contents/MacOS/PADDesktop"
  assert_arm64_only "${resources}/bin/bun"
  assert_internal_symlinks "${resources}"
  assert_bundle_minos_at_most "${app_bundle}" "${MINIMUM_MACOS_VERSION}"
  pi_version="$(extract_semver "$(/usr/bin/env -i HOME="${TMPDIR:-/tmp}" PATH="/usr/bin:/bin:/usr/sbin:/sbin" LANG="en_US.UTF-8" "${resources}/bin/pi" --version)")" || fail "Pi runtime did not report a semantic version"
  bun_version="$(extract_semver "$("${resources}/bin/bun" --version)")" || fail "Bun runtime did not report a semantic version"
  [[ "${pi_version}" == "${EXPECTED_PI_VERSION}" ]] || fail "Pi must be ${EXPECTED_PI_VERSION}, got ${pi_version}"
  [[ "${bun_version}" == "${EXPECTED_BUN_VERSION}" ]] || fail "Bun must be ${EXPECTED_BUN_VERSION}, got ${bun_version}"
  /usr/bin/python3 - "${resources}/pi/package.json" "${EXPECTED_PI_VERSION}" <<'PY'
import json
import pathlib
import sys

metadata = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if metadata.get("name") != "@earendil-works/pi-coding-agent" or metadata.get("version") != sys.argv[2]:
    raise SystemExit("bundled Pi package metadata does not match the pinned runtime")
PY
  verify_runtime_evidence "${resources}"
  /usr/bin/codesign --verify --deep --strict --verbose=2 "${app_bundle}"
  signature_details="$(/usr/bin/codesign -d --verbose=4 "${app_bundle}" 2>&1)"
  if [[ "${signature_details}" == *'Signature=adhoc'* ]]; then
    :
  elif [[ "${signature_details}" == *'Authority=Developer ID Application:'* && "${signature_details}" == *runtime* ]]; then
    :
  else
    fail "bundle must be either local ad-hoc or hardened Developer ID signed"
  fi
  entitlements="$(mktemp -t pad-desktop-entitlements)"
  /usr/bin/codesign -d --entitlements :- "${app_bundle}" >"${entitlements}" 2>/dev/null
  [[ "$(plist_value "${entitlements}" com.apple.security.cs.allow-jit)" == true ]] || fail "missing JIT entitlement"
  if [[ "${signature_details}" == *'Signature=adhoc'* ]]; then
    [[ "$(plist_value "${entitlements}" com.apple.security.cs.disable-library-validation)" == true ]] || fail "missing ad-hoc library validation entitlement"
  elif plist_value "${entitlements}" com.apple.security.cs.disable-library-validation >/dev/null; then
    fail "Developer ID app unexpectedly disables library validation"
  fi
  /bin/rm -f -- "${entitlements}"
}

running_target_pids() {
  /bin/ps -axo pid=,comm= | /usr/bin/awk -v prefix="${TARGET_APP}/" 'index($0, prefix) { print $1 }'
}

health_related_pids() {
  /bin/ps -axo pid=,command= | /usr/bin/awk \
    -v app="${HEALTH_APP}/Contents/MacOS/PADDesktop" \
    'index($0, app) && !index($0, "/usr/bin/awk") { print $1 }'
}

process_group_alive() {
  [[ -n "${HEALTH_PROCESS_GROUP}" ]] || return 1
  /bin/kill -0 -- "-${HEALTH_PROCESS_GROUP}" >/dev/null 2>&1
}

cleanup_health_process() {
  local related
  if [[ -z "${HEALTH_PID}" && -z "${HEALTH_PROCESS_GROUP}" ]]; then
    return
  fi
  if process_group_alive; then
    /bin/kill -TERM -- "-${HEALTH_PROCESS_GROUP}" >/dev/null 2>&1 || true
    for _attempt in {1..30}; do
      process_group_alive || break
      /bin/sleep 0.1
    done
  fi
  if process_group_alive; then
    /bin/kill -KILL -- "-${HEALTH_PROCESS_GROUP}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${HEALTH_PID}" ]]; then
    wait "${HEALTH_PID}" >/dev/null 2>&1 || true
  fi
  related="$(health_related_pids)"
  if [[ -n "${related}" ]]; then
    while IFS= read -r pid; do
      [[ "${pid}" == <-> ]] && /bin/kill -KILL "${pid}" >/dev/null 2>&1 || true
    done <<<"${related}"
  fi
  HEALTH_PID=""
  HEALTH_PROCESS_GROUP=""
}

free_tcp_port() {
  /usr/bin/python3 - <<'PY'
import socket

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
    listener.bind(("127.0.0.1", 0))
    print(listener.getsockname()[1])
PY
}

wait_for_cdp_target() {
  local port="$1"
  local target_file="$2"
  local websocket_file="$3"
  for _attempt in {1..200}; do
    if [[ -n "${HEALTH_PID}" ]] && ! /bin/kill -0 "${HEALTH_PID}" >/dev/null 2>&1; then
      return 1
    fi
    if /usr/bin/curl --silent --show-error --fail --max-time 1 \
      "http://127.0.0.1:${port}/json/list" >"${target_file}" 2>/dev/null; then
      if /usr/bin/python3 - "${target_file}" "${websocket_file}" <<'PY'
import json
import pathlib
import sys

targets = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
# Electron exposes the page target before its first navigation.  Attaching to
# that transient about:blank target makes Runtime.evaluate lose its context
# and can look like a renderer startup timeout. Wait for PAD's stable custom
# protocol URL instead.
pages = [
    item for item in targets
    if item.get("type") == "page"
    and item.get("webSocketDebuggerUrl")
    and str(item.get("url", "")).startswith("pad-app://renderer/")
]
if not pages:
    raise SystemExit(1)
pathlib.Path(sys.argv[2]).write_text(str(pages[0]["webSocketDebuggerUrl"]), encoding="utf-8")
PY
      then
        return 0
      fi
    fi
    /bin/sleep 0.1
  done
  return 1
}

run_cdp_health_evaluation() {
  local websocket_url="$1"
  local result_file="$2"
  local probe_script

  probe_script="$(<<'JAVASCRIPT'
const url = process.env.PAD_CDP_WS;
if (!url) throw new Error("missing PAD_CDP_WS");
const socket = new WebSocket(url);
let sequence = 0;
const pending = new Map();
function call(method, params = {}) {
  return new Promise((resolve, reject) => {
    const id = ++sequence;
    // A cold packaged Bun/Pi runtime can still be compiling its first
    // provider module while the renderer is already reachable. Keep the
    // protocol probe alive for the same bounded window as the readiness loop.
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`CDP timeout: ${method}`));
    }, 60000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method, params }));
  });
}
socket.addEventListener("message", (event) => {
  const message = JSON.parse(String(event.data));
  const item = pending.get(message.id);
  if (!item) return;
  clearTimeout(item.timer);
  pending.delete(message.id);
  if (message.error) item.reject(new Error(JSON.stringify(message.error)));
  else item.resolve(message.result);
});
socket.addEventListener("open", async () => {
  try {
    const expression = `(async () => {
      const delay = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
      let shell = null;
      // The first Pi ModelRuntime catalog read may trigger Bun's lazy
      // provider transpilation on a cold machine. Give the signed bundle a
      // generous window so install verification does not race that startup.
      for (let attempt = 0; attempt < 600; attempt += 1) {
        shell = document.querySelector(".app-shell");
        if (document.readyState === "complete"
          && shell
          && !shell.classList.contains("is-loading")
          && !document.querySelector(".task-data-loading")
          && typeof window.padDesktop?.bootstrap === "function") break;
        await delay(50);
      }
      if (!shell || shell.classList.contains("is-loading")) {
        throw new Error("renderer did not reach its ready shell");
      }
      const bootstrap = await window.padDesktop.bootstrap();
      const ping = await window.padDesktop.request("ping", {});
      const visible = (element) => {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return style.display !== "none" && style.visibility !== "hidden"
          && Number(style.opacity || "1") > 0 && rect.width > 0 && rect.height > 0;
      };
      const alerts = [...document.querySelectorAll('[role="alert"]')]
        .filter(visible)
        .map((element) => (element.textContent || "").trim())
        .filter(Boolean);
      return {
        rendererReady: true,
        rendererUrl: location.href,
        protocolVersion: bootstrap?.protocol_version ?? null,
        pingProtocolVersion: ping?.protocol_version ?? null,
        backendStatus: bootstrap?.backend?.status ?? null,
        alerts,
      };
    })()`;
    const response = await call("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      userGesture: true,
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
setTimeout(() => {
  console.error("health evaluation timed out");
  process.exit(1);
}, 90000).unref();
JAVASCRIPT
)"
  PAD_CDP_WS="${websocket_url}" "${HEALTH_APP}/Contents/Resources/bin/bun" -e "${probe_script}" >"${result_file}"
  /usr/bin/python3 - "${result_file}" <<'PY'
import json
import pathlib
import sys

result = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if result.get("rendererReady") is not True:
    raise SystemExit("renderer is not ready")
if result.get("rendererUrl") != "pad-app://renderer/index.html":
    raise SystemExit(f"unexpected renderer URL: {result.get('rendererUrl')}")
if result.get("protocolVersion") != 2 or result.get("pingProtocolVersion") != 2:
    raise SystemExit("renderer/backend did not negotiate protocol v2")
if result.get("backendStatus") != "ready":
    raise SystemExit(f"backend is not ready: {result.get('backendStatus')}")
if result.get("alerts"):
    raise SystemExit(f"renderer exposed fatal alerts: {result.get('alerts')}")
PY
}

close_health_app_over_cdp() {
  local websocket_url="$1"
  local close_script
  close_script='const url=process.env.PAD_CDP_WS; const socket=new WebSocket(url); socket.addEventListener("open",()=>socket.send(JSON.stringify({id:1,method:"Browser.close"}))); socket.addEventListener("message",()=>{socket.close();process.exit(0)}); setTimeout(()=>process.exit(0),1500);'
  PAD_CDP_WS="${websocket_url}" "${HEALTH_APP}/Contents/Resources/bin/bun" -e "${close_script}" >/dev/null 2>&1 || true
}

run_isolated_health_probe() {
  local executable="${HEALTH_APP}/Contents/MacOS/PADDesktop"
  local health_root="${INSTALL_STAGE}/health"
  local health_home="${health_root}/home"
  local health_data="${health_root}/data"
  local health_user_data="${health_root}/electron-user-data"
  local port
  local websocket_url
  local related
  local exit_status=0

  related="$(health_related_pids)"
  [[ -z "${related}" ]] || fail "refusing an ambiguous health probe while this app bundle is already running: ${(j:,:)${(f)related}}"
  mkdir -p "${health_home}" "${health_data}" "${health_user_data}"
  chmod 700 "${health_root}" "${health_home}" "${health_data}" "${health_user_data}"
  port="$(free_tcp_port)"

  /usr/bin/python3 - \
    "${executable}" \
    "${health_home}" \
    "${health_data}" \
    "${health_user_data}" \
    "${port}" \
    >"${health_root}/stdout.log" \
    2>"${health_root}/stderr.log" <<'PY' &
import os
import sys

executable, home, data_root, user_data, port = sys.argv[1:]
environment = {
    "HOME": home,
    "USER": os.environ.get("USER", "pad"),
    "LOGNAME": os.environ.get("LOGNAME", "pad"),
    "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
    "LANG": os.environ.get("LANG", "en_US.UTF-8"),
    "TMPDIR": os.environ.get("TMPDIR", "/tmp"),
    "PAD_DESKTOP_DATA_DIR": data_root,
    "ELECTRON_ENABLE_LOGGING": "1",
}
os.setsid()
os.execve(
    executable,
    [
        executable,
        f"--remote-debugging-port={port}",
        "--remote-allow-origins=*",
        f"--user-data-dir={user_data}",
        "--no-first-run",
        "--disable-default-apps",
        "--window-size=1280,820",
        "--window-position=-10000,-10000",
    ],
    environment,
)
PY
  HEALTH_PID=$!
  HEALTH_PROCESS_GROUP="${HEALTH_PID}"

  if ! wait_for_cdp_target "${port}" "${health_root}/targets.json" "${health_root}/websocket.txt"; then
    /usr/bin/tail -n 80 "${health_root}/stderr.log" >&2 || true
    fail "installed app did not expose a renderer during the isolated health probe"
  fi
  websocket_url="$(<"${health_root}/websocket.txt")"
  if ! run_cdp_health_evaluation "${websocket_url}" "${health_root}/result.json"; then
    /usr/bin/tail -n 80 "${health_root}/stderr.log" >&2 || true
    fail "installed renderer/backend protocol-v2 health probe failed"
  fi

  close_health_app_over_cdp "${websocket_url}"
  /bin/kill -TERM "${HEALTH_PID}" >/dev/null 2>&1 || true
  for _attempt in {1..120}; do
    /bin/kill -0 "${HEALTH_PID}" >/dev/null 2>&1 || break
    /bin/sleep 0.1
  done
  if /bin/kill -0 "${HEALTH_PID}" >/dev/null 2>&1; then
    fail "installed app did not exit cleanly after its health probe"
  fi
  wait "${HEALTH_PID}" || exit_status=$?
  [[ "${exit_status}" -eq 0 ]] || fail "installed app health process exited with status ${exit_status}"

  for _attempt in {1..50}; do
    related="$(health_related_pids)"
    process_group_alive || [[ -n "${related}" ]] || break
    /bin/sleep 0.1
  done
  related="$(health_related_pids)"
  [[ -z "${related}" ]] || fail "installed app left health-probe processes running: ${(j:,:)${(f)related}}"
  process_group_alive && fail "installed app process group survived clean shutdown"

  HEALTH_COMPLETE=1
  HEALTH_PID=""
  HEALTH_PROCESS_GROUP=""
  echo "INSTALL_HEALTH_OK: renderer ready, Electron local backend ready, protocol v2, no fatal alerts, clean shutdown"
}

verify_bundle "${SOURCE_APP}"
if (( CHECK_ONLY )); then
  INSTALL_STAGE="$(mktemp -d /tmp/pad-desktop-install-check.XXXXXX)"
  HEALTH_APP="${SOURCE_APP}"
  run_isolated_health_probe
  echo "Verified install source, including runtime health, without changing /Applications: ${SOURCE_APP}"
  INSTALL_COMPLETE=1
  exit 0
fi

[[ -w /Applications ]] || fail "/Applications is not writable by the current user"
if [[ -e "${TARGET_APP}" ]]; then
  [[ -d "${TARGET_APP}" && ! -L "${TARGET_APP}" ]] || fail "refusing to replace a non-directory or symlink at ${TARGET_APP}"
  [[ "$(plist_value "${TARGET_APP}/Contents/Info.plist" CFBundleIdentifier)" == cn.ghostcloud.pad.desktop ]] || fail "refusing to replace an app with a different bundle identifier"
fi

TARGET_PIDS="$(running_target_pids)"
if [[ -n "${TARGET_PIDS}" ]]; then
  /usr/bin/osascript -e 'tell application id "cn.ghostcloud.pad.desktop" to quit' >/dev/null 2>&1 || true
  for _attempt in {1..40}; do
    [[ -z "$(running_target_pids)" ]] && break
    /bin/sleep 0.25
  done
fi
TARGET_PIDS="$(running_target_pids)"
[[ -z "${TARGET_PIDS}" ]] || fail "PAD Desktop is still running (PIDs: ${(j:,:)${(f)TARGET_PIDS}}); installation was not started"

BACKUP_DIR="${PAD_DESKTOP_BACKUP_DIR:-${APP_DIR}/release/backups}"
BACKUP_DIR="$(/usr/bin/python3 -c 'import os, sys; print(os.path.abspath(sys.argv[1]))' "${BACKUP_DIR}")"
case "${BACKUP_DIR}" in
  /|/Applications|"${HOME}"|"${TARGET_APP}"|"${APP_DIR}") fail "refusing unsafe backup directory: ${BACKUP_DIR}" ;;
esac
if [[ -e "${BACKUP_DIR}" && -L "${BACKUP_DIR}" ]]; then
  fail "backup directory must not be a symlink: ${BACKUP_DIR}"
fi
mkdir -p "${BACKUP_DIR}"
[[ -d "${BACKUP_DIR}" && ! -L "${BACKUP_DIR}" ]] || fail "invalid backup directory: ${BACKUP_DIR}"

TIMESTAMP="$(/bin/date -u +%Y%m%dT%H%M%SZ)"
FAILED_NEW_APP="${BACKUP_DIR}/PAD Desktop-failed-new-${TIMESTAMP}-$$.app"
INSTALL_STAGE="$(mktemp -d /Applications/.pad-desktop-install.XXXXXX)"
STAGED_APP="${INSTALL_STAGE}/PAD Desktop.app"
/usr/bin/ditto "${SOURCE_APP}" "${STAGED_APP}"
verify_bundle "${STAGED_APP}"

if [[ -d "${TARGET_APP}" ]]; then
  OLD_VERSION="$(plist_value "${TARGET_APP}/Contents/Info.plist" CFBundleShortVersionString || echo unknown)"
  OLD_VERSION="${OLD_VERSION//[^A-Za-z0-9._-]/_}"
  BACKUP_APP="${BACKUP_DIR}/PAD Desktop-before-${TIMESTAMP}-${OLD_VERSION}.app"
  [[ ! -e "${BACKUP_APP}" ]] || fail "backup already exists: ${BACKUP_APP}"
  /bin/mv "${TARGET_APP}" "${BACKUP_APP}"
fi

INSTALL_STARTED=1
/bin/mv "${STAGED_APP}" "${TARGET_APP}"
HEALTH_APP="${TARGET_APP}"
verify_bundle "${TARGET_APP}"
run_isolated_health_probe

echo "Installed PAD Desktop at: ${TARGET_APP}"
if [[ -n "${BACKUP_APP}" ]]; then
  echo "Previous app backup: ${BACKUP_APP}"
fi
if (( LAUNCH_AFTER_INSTALL )); then
  /usr/bin/open "${TARGET_APP}" || fail "installed app passed health checks but could not be launched"
  echo "Launched PAD Desktop."
fi
INSTALL_COMPLETE=1
echo "INSTALL_COMPLETE: static verification and isolated runtime health probe passed"
