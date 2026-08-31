#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="${0:A:h}"
APP_DIR="${SCRIPT_DIR:h}"
PACKAGE_STAGE="$(mktemp -d /tmp/pad-electron-package.XXXXXX)"
RESOURCE_STAGE="${PACKAGE_STAGE}/resources"
ELECTRON_ZIP_STAGE="${PACKAGE_STAGE}/electron-zip"
SMOKE_HOME="${PACKAGE_STAGE}/smoke-home"
EVIDENCE_STAGE="${RESOURCE_STAGE}/release-evidence"
EXPECTED_PI_VERSION="0.84.4"
EXPECTED_BUN_VERSION="1.3.14"
EXPECTED_NODE_VERSION="24.20.0"
EXPECTED_RUNTIME_NODE_VERSION="22.19.0"
MINIMUM_MACOS_VERSION="13.0"
SIGN_IDENTITY="${PAD_DESKTOP_SIGN_IDENTITY:-}"
SIGNING_MODE="ad-hoc-local-only"

if [[ -n "${SIGN_IDENTITY}" ]]; then
  [[ "${SIGN_IDENTITY}" == 'Developer ID Application:'* ]] || {
    echo "PAD Desktop package error: PAD_DESKTOP_SIGN_IDENTITY must name a Developer ID Application certificate" >&2
    exit 1
  }
  SIGNING_MODE="developer-id"
fi

fail() {
  echo "PAD Desktop package error: $*" >&2
  exit 1
}

cleanup_package_stage() {
  if [[ -n "${PACKAGE_STAGE:-}" && -d "${PACKAGE_STAGE}" && "${PACKAGE_STAGE}" == /tmp/pad-electron-package.* ]]; then
    rm -rf -- "${PACKAGE_STAGE}"
  fi
}
trap cleanup_package_stage EXIT

PACKAGE_NODE="$("${SCRIPT_DIR}/run-electron-forge.sh" --print-node)" || fail "pinned Node ${EXPECTED_NODE_VERSION} is unavailable"
[[ -x "${PACKAGE_NODE}" ]] || fail "pinned Node executable is invalid: ${PACKAGE_NODE}"
[[ "$("${PACKAGE_NODE}" -p 'process.versions.node')" == "${EXPECTED_NODE_VERSION}" ]] || fail "package Node must be ${EXPECTED_NODE_VERSION}"
RUNTIME_NODE="$(npx --offline --yes "node@${EXPECTED_RUNTIME_NODE_VERSION}" -p 'process.execPath' 2>/dev/null)" \
  || fail "cached runtime Node ${EXPECTED_RUNTIME_NODE_VERSION} is unavailable"
[[ -x "${RUNTIME_NODE}" ]] || fail "runtime Node executable is invalid: ${RUNTIME_NODE}"
[[ "$("${RUNTIME_NODE}" -p 'process.versions.node')" == "${EXPECTED_RUNTIME_NODE_VERSION}" ]] \
  || fail "runtime Node must be ${EXPECTED_RUNTIME_NODE_VERSION}"
export PAD_FORGE_NODE_BIN="${PACKAGE_NODE}"

mkdir -p \
  "${RESOURCE_STAGE}/bin" \
  "${RESOURCE_STAGE}/pi" \
  "${EVIDENCE_STAGE}" \
  "${ELECTRON_ZIP_STAGE}" \
  "${SMOKE_HOME}"
chmod 700 "${PACKAGE_STAGE}" "${SMOKE_HOME}"

real_path() {
  /usr/bin/python3 -c 'import os, sys; print(os.path.realpath(sys.argv[1]))' "$1"
}

assert_arm64_only() {
  local binary="$1"
  local architectures
  [[ -x "${binary}" ]] || fail "required executable is missing: ${binary}"
  architectures="$(/usr/bin/lipo -archs "${binary}" 2>/dev/null)" || fail "not a Mach-O executable: ${binary}"
  [[ "${architectures}" == "arm64" ]] || fail "expected arm64-only executable, got '${architectures}': ${binary}"
}

assert_system_linkage() {
  local binary="$1"
  local dependency
  while IFS= read -r dependency; do
    case "${dependency}" in
      /usr/lib/*|/System/Library/*|@*) ;;
      *) fail "external dynamic library dependency in ${binary}: ${dependency}" ;;
    esac
  done < <(/usr/bin/otool -L "${binary}" | /usr/bin/sed -n '2,$p' | /usr/bin/awk '{print $1}')
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
import os
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
checked: list[tuple[str, str]] = []
required_suffixes = {
    "Contents/MacOS/PADDesktop",
    "Contents/Resources/bin/bun",
    "Contents/Resources/bin/node",
    "Contents/Frameworks/Electron Framework.framework/Versions/A/Electron Framework",
    "Contents/Frameworks/PAD Desktop Helper.app/Contents/MacOS/PAD Desktop Helper",
    "Contents/Frameworks/PAD Desktop Helper (GPU).app/Contents/MacOS/PAD Desktop Helper (GPU)",
    "Contents/Frameworks/PAD Desktop Helper (Renderer).app/Contents/MacOS/PAD Desktop Helper (Renderer)",
    "Contents/Frameworks/PAD Desktop Helper (Plugin).app/Contents/MacOS/PAD Desktop Helper (Plugin)",
}
seen_suffixes: set[str] = set()

for candidate in sorted(bundle.rglob("*")):
    try:
        metadata = candidate.stat()
    except OSError:
        continue
    if not stat.S_ISREG(metadata.st_mode) or not metadata.st_mode & 0o111:
        continue
    kind = subprocess.run(
        ["/usr/bin/file", "-b", str(candidate)],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout
    if not kind.startswith("Mach-O"):
        continue
    loads = subprocess.run(
        ["/usr/bin/otool", "-m", "-l", str(candidate)],
        check=True,
        text=True,
        stdout=subprocess.PIPE,
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
    if relative in required_suffixes:
        seen_suffixes.add(relative)
    if not minimums:
        raise SystemExit(f"Mach-O has no macOS deployment target: {relative}")
    for minimum in minimums:
        if version(minimum) > maximum:
            raise SystemExit(
                f"Mach-O requires macOS {minimum}, above Info.plist {maximum_text}: {relative}"
            )
        checked.append((relative, minimum))

missing = sorted(required_suffixes - seen_suffixes)
if missing:
    raise SystemExit(f"critical Mach-O files were not checked: {', '.join(missing)}")
if len(checked) < len(required_suffixes):
    raise SystemExit("too few Mach-O deployment targets were checked")
print(f"verified {len(checked)} Mach-O deployment targets <= macOS {maximum_text}")
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
        resolved = os.path.realpath(candidate)
        if os.path.commonpath((root, resolved)) != root:
            raise SystemExit(f"escaping symlink is not allowed: {candidate} -> {link}")
PY
}

assert_plist_value() {
  local plist="$1"
  local key="$2"
  local expected="$3"
  local actual
  actual="$(/usr/libexec/PlistBuddy -c "Print :${key}" "${plist}" 2>/dev/null)" || fail "missing plist key ${key}"
  [[ "${actual}" == "${expected}" ]] || fail "unexpected ${key}: '${actual}' (expected '${expected}')"
}

verify_fuses() {
  PAD_FUSE_APP="$1" PAD_FUSE_MODULE="${APP_DIR}/node_modules/@electron/fuses" "${PACKAGE_NODE}" <<'NODE'
const { getCurrentFuseWire } = require(process.env.PAD_FUSE_MODULE);

const expected = [0x30, 0x30, 0x30, 0x30, 0x31, 0x31, 0x30, 0x30, 0x31];
getCurrentFuseWire(process.env.PAD_FUSE_APP).then((wire) => {
  if (wire.version !== '1') throw new Error(`unexpected fuse version: ${wire.version}`);
  const actual = Object.keys(wire)
    .filter((key) => /^\d+$/.test(key))
    .map(Number)
    .sort((left, right) => left - right)
    .map((key) => wire[key]);
  if (actual.length !== expected.length || actual.some((value, index) => value !== expected[index])) {
    throw new Error(`unexpected Electron fuse wire: ${JSON.stringify(actual)}`);
  }
}).catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
NODE
}

verify_auth_import_contract() {
  local resources="$1"
  local marker

  # Keep this import exactly aligned with the Electron authentication helper.
  # This deliberately stops before
  # ModelRuntime.create(): no provider is contacted and no auth files or
  # credentials are created.  Running from the packaged Pi root also proves
  # that bare-package resolution works through the bundled Node runtime used
  # by the TypeScript authentication helper.
  marker="$(
    cd "${resources}/pi"
    /usr/bin/env -i \
      HOME="${SMOKE_HOME}" \
      USER="${USER:-pad}" \
      LOGNAME="${LOGNAME:-pad}" \
      PATH="/usr/bin:/bin:/usr/sbin:/sbin" \
      LANG="en_US.UTF-8" \
      TMPDIR="${TMPDIR:-/tmp}" \
      "${resources}/bin/node" --input-type=module -e '
        import { ModelRuntime } from "@earendil-works/pi-coding-agent";
        if (typeof ModelRuntime !== "function") {
          throw new Error(`ModelRuntime named export has unexpected type: ${typeof ModelRuntime}`);
        }
        if (typeof ModelRuntime.create !== "function") {
          throw new Error("ModelRuntime.create is missing from the named export");
        }
        process.stdout.write("PAD_AUTH_IMPORT_OK");
      '
  )" || fail "bundled Pi authentication import contract smoke test failed"
  [[ "${marker}" == "PAD_AUTH_IMPORT_OK" ]] || fail "unexpected authentication import smoke output: ${marker}"
}

generate_runtime_evidence() {
  local app_version="$1"
  local electron_version="$2"
  local electron_zip_sha="$3"
  local created_at
  local checksum_sha

  created_at="$(/bin/date -u +%Y-%m-%dT%H:%M:%SZ)"
  /usr/bin/python3 - \
    "${EVIDENCE_STAGE}/runtime-sbom.spdx.json" \
    "${app_version}" \
    "${EXPECTED_PI_VERSION}" \
    "${EXPECTED_BUN_VERSION}" \
    "${EXPECTED_RUNTIME_NODE_VERSION}" \
    "${electron_version}" \
    "${created_at}" <<'PY'
import hashlib
import json
import pathlib
import sys

destination = pathlib.Path(sys.argv[1])
app_version, pi_version, bun_version, node_version, electron_version, created_at = sys.argv[2:]
seed = "|".join((app_version, pi_version, bun_version, node_version, electron_version))
namespace = "https://ghostcloud.cn/spdx/pad-desktop/runtime/" + hashlib.sha256(seed.encode()).hexdigest()
packages = [
    ("SPDXRef-Package-PAD", "PAD Desktop control plane", app_version),
    ("SPDXRef-Package-Pi", "@earendil-works/pi-coding-agent", pi_version),
    ("SPDXRef-Package-Bun", "Bun", bun_version),
    ("SPDXRef-Package-Node", "Node.js", node_version),
    ("SPDXRef-Package-Electron", "Electron", electron_version),
]
document = {
    "spdxVersion": "SPDX-2.3",
    "dataLicense": "CC0-1.0",
    "SPDXID": "SPDXRef-DOCUMENT",
    "name": f"PAD-Desktop-{app_version}-runtime",
    "documentNamespace": namespace,
    "creationInfo": {
        "created": created_at,
        "creators": ["Tool: PAD Desktop package-electron-app.sh"],
    },
    "packages": [
        {
            "name": name,
            "SPDXID": spdx_id,
            "versionInfo": package_version,
            "downloadLocation": "NOASSERTION",
            "filesAnalyzed": False,
            "licenseConcluded": "NOASSERTION",
            "licenseDeclared": "NOASSERTION",
            "copyrightText": "NOASSERTION",
        }
        for spdx_id, name, package_version in packages
    ],
    "relationships": [
        {
            "spdxElementId": "SPDXRef-DOCUMENT",
            "relationshipType": "DESCRIBES",
            "relatedSpdxElement": spdx_id,
        }
        for spdx_id, _name, _version in packages
    ],
}
destination.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

  (
    cd "${RESOURCE_STAGE}"
    /usr/bin/shasum -a 256 \
      bin/bun \
      bin/node \
      bin/pi \
      pi/package.json \
      pi/dist/bun/cli.js \
      pi/dist/bundle/cli.js \
      release-evidence/runtime-sbom.spdx.json \
      > release-evidence/runtime-SHA256SUMS.txt
  )
  checksum_sha="$(/usr/bin/shasum -a 256 "${EVIDENCE_STAGE}/runtime-SHA256SUMS.txt" | /usr/bin/awk '{print $1}')"

  /usr/bin/python3 - \
    "${EVIDENCE_STAGE}/runtime-manifest.json" \
    "${app_version}" \
    "${EXPECTED_PI_VERSION}" \
    "${EXPECTED_BUN_VERSION}" \
    "${EXPECTED_RUNTIME_NODE_VERSION}" \
    "${electron_version}" \
    "${electron_zip_sha}" \
    "${checksum_sha}" \
    "${SIGNING_MODE}" \
    "${created_at}" <<'PY'
import json
import pathlib
import sys

(
    destination,
    app_version,
    pi_version,
    bun_version,
    node_version,
    electron_version,
    electron_zip_sha,
    checksum_sha,
    signing_mode,
    created_at,
) = sys.argv[1:]
manifest = {
    "schema": "cn.ghostcloud.pad.desktop.runtime-evidence.v1",
    "created_at": created_at,
    "target": "darwin-arm64",
    "minimum_macos": "13.0",
    "signing_mode": signing_mode,
    "components": [
        {"name": "PAD Desktop Electron control plane", "version": app_version, "runtime_path": "app.asar"},
        {"name": "Pi coding agent", "version": pi_version, "runtime_path": "pi/package.json"},
        {"name": "Bun", "version": bun_version, "runtime_path": "bin/bun"},
        {"name": "Node.js", "version": node_version, "runtime_path": "bin/node"},
        {
            "name": "Electron",
            "version": electron_version,
            "source_archive_sha256": electron_zip_sha,
        },
    ],
    "checksums": {
        "algorithm": "SHA-256",
        "manifest": "runtime-SHA256SUMS.txt",
        "manifest_sha256": checksum_sha,
    },
    "sbom": {"format": "SPDX-2.3", "path": "runtime-sbom.spdx.json"},
}
pathlib.Path(destination).write_text(
    json.dumps(manifest, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
PY
}

verify_runtime_evidence() {
  local resources="$1"
  (
    cd "${resources}"
    /usr/bin/shasum -a 256 -c release-evidence/runtime-SHA256SUMS.txt >/dev/null
  ) || fail "runtime checksum evidence does not match bundled files"
  /usr/bin/python3 - \
    "${resources}/release-evidence/runtime-manifest.json" \
    "${resources}/release-evidence/runtime-sbom.spdx.json" \
    "${EXPECTED_PI_VERSION}" \
    "${EXPECTED_BUN_VERSION}" \
    "${EXPECTED_RUNTIME_NODE_VERSION}" \
    "${SIGNING_MODE}" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
sbom = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
pi_version, bun_version, node_version, signing_mode = sys.argv[3:]
if manifest.get("schema") != "cn.ghostcloud.pad.desktop.runtime-evidence.v1":
    raise SystemExit("invalid runtime evidence schema")
if manifest.get("minimum_macos") != "13.0" or manifest.get("target") != "darwin-arm64":
    raise SystemExit("invalid runtime target evidence")
if manifest.get("signing_mode") != signing_mode:
    raise SystemExit("runtime signing evidence does not match build mode")
versions = {item.get("name"): item.get("version") for item in manifest.get("components", [])}
if versions.get("Pi coding agent") != pi_version or versions.get("Bun") != bun_version or versions.get("Node.js") != node_version:
    raise SystemExit("runtime version evidence does not match pinned versions")
if sbom.get("spdxVersion") != "SPDX-2.3" or sbom.get("SPDXID") != "SPDXRef-DOCUMENT":
    raise SystemExit("invalid SPDX runtime SBOM")
package_versions = {item.get("name"): item.get("versionInfo") for item in sbom.get("packages", [])}
if package_versions.get("@earendil-works/pi-coding-agent") != pi_version:
    raise SystemExit("Pi is missing or unpinned in runtime SBOM")
if package_versions.get("Bun") != bun_version:
    raise SystemExit("Bun is missing or unpinned in runtime SBOM")
if package_versions.get("Node.js") != node_version:
    raise SystemExit("Node.js is missing or unpinned in runtime SBOM")
PY
}

refresh_packaged_runtime_evidence() {
  local app_bundle="$1"
  local resources="${app_bundle}/Contents/Resources"
  local evidence="${resources}/release-evidence"
  local entitlements="${PACKAGE_STAGE}/final-app-entitlements.plist"
  local checksum_sha

  # electron-osx-sign signs the copied PAD, Node and Bun Mach-O files. Their final
  # CodeDirectory bytes therefore differ from the unsigned staging copies.
  # Recompute evidence from the signed bundle, update the manifest, and then
  # re-seal only the top-level app; all nested signatures remain unchanged.
  (
    cd "${resources}"
    /usr/bin/shasum -a 256 \
      bin/bun \
      bin/node \
      bin/pi \
      pi/package.json \
      pi/dist/bun/cli.js \
      pi/dist/bundle/cli.js \
      release-evidence/runtime-sbom.spdx.json \
      > release-evidence/runtime-SHA256SUMS.txt
  )
  checksum_sha="$(/usr/bin/shasum -a 256 "${evidence}/runtime-SHA256SUMS.txt" | /usr/bin/awk '{print $1}')"
  /usr/bin/python3 - "${evidence}/runtime-manifest.json" "${checksum_sha}" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
manifest = json.loads(path.read_text(encoding="utf-8"))
manifest["checksums"]["manifest_sha256"] = sys.argv[2]
path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

  /usr/bin/codesign -d --entitlements :- "${app_bundle}" >"${entitlements}" 2>/dev/null
  /usr/bin/plutil -lint "${entitlements}" >/dev/null
  if [[ "${SIGNING_MODE}" == developer-id ]]; then
    /usr/bin/codesign \
      --force \
      --sign "${SIGN_IDENTITY}" \
      --options runtime \
      --timestamp \
      --entitlements "${entitlements}" \
      "${app_bundle}"
  else
    /usr/bin/codesign \
      --force \
      --sign - \
      --options runtime \
      --timestamp=none \
      --entitlements "${entitlements}" \
      "${app_bundle}"
  fi
}

verify_app_bundle() {
  local app_bundle="$1"
  local resources="${app_bundle}/Contents/Resources"
  local info_plist="${app_bundle}/Contents/Info.plist"
  local entitlements_plist="${PACKAGE_STAGE}/app-entitlements.plist"
  local bun_version
  local node_version
  local pi_version
  local signature_details

  [[ -d "${app_bundle}" && ! -L "${app_bundle}" ]] || fail "app bundle is missing or is a symlink: ${app_bundle}"
  /usr/bin/plutil -lint "${info_plist}" >/dev/null
  assert_plist_value "${info_plist}" CFBundleIdentifier cn.ghostcloud.pad.desktop
  assert_plist_value "${info_plist}" CFBundleExecutable PADDesktop
  assert_plist_value "${info_plist}" CFBundleDisplayName 'PAD Desktop'
  assert_plist_value "${info_plist}" CFBundleName 'PAD Desktop'
  assert_plist_value "${info_plist}" LSMinimumSystemVersion 13.0
  for forbidden_key in \
    NSAppTransportSecurity \
    NSAudioCaptureUsageDescription \
    NSBluetoothAlwaysUsageDescription \
    NSBluetoothPeripheralUsageDescription \
    NSCameraUsageDescription \
    NSMicrophoneUsageDescription; do
    if /usr/libexec/PlistBuddy -c "Print :${forbidden_key}" "${info_plist}" >/dev/null 2>&1; then
      fail "unexpected privacy/network entitlement description in Info.plist: ${forbidden_key}"
    fi
  done

  [[ -f "${resources}/app.asar" ]] || fail "app.asar is missing"
  [[ -f "${resources}/pi/package.json" ]] || fail "bundled Pi package.json is missing"
  [[ -f "${resources}/pi/dist/bun/cli.js" ]] || fail "bundled Pi Bun entrypoint is missing"
  [[ -f "${resources}/pi/dist/bundle/cli.js" ]] || fail "bundled Pi Node entrypoint is missing"
  for evidence in \
    runtime-manifest.json \
    runtime-sbom.spdx.json \
    runtime-SHA256SUMS.txt; do
    [[ -f "${resources}/release-evidence/${evidence}" ]] || fail "runtime evidence is missing: ${evidence}"
  done
  for executable in \
    "${app_bundle}/Contents/MacOS/PADDesktop" \
    "${resources}/bin/bun" \
    "${resources}/bin/node" \
    "${resources}/bin/pi"; do
    [[ -x "${executable}" ]] || fail "bundled executable is missing: ${executable}"
  done

  assert_arm64_only "${app_bundle}/Contents/MacOS/PADDesktop"
  assert_arm64_only "${resources}/bin/bun"
  assert_system_linkage "${resources}/bin/bun"
  assert_arm64_only "${resources}/bin/node"
  assert_system_linkage "${resources}/bin/node"
  assert_internal_symlinks "${resources}"
  assert_bundle_minos_at_most "${app_bundle}" "${MINIMUM_MACOS_VERSION}"

  "${PACKAGE_NODE}" -e '
    const value = require(process.argv[1]);
    if (value.name !== "@earendil-works/pi-coding-agent" || value.version !== process.argv[2]) {
      throw new Error("invalid bundled Pi package metadata");
    }
  ' "${resources}/pi/package.json" "${EXPECTED_PI_VERSION}"
  pi_version="$(/usr/bin/env -i \
    HOME="${SMOKE_HOME}" \
    USER="${USER:-pad}" \
    LOGNAME="${LOGNAME:-pad}" \
    PATH="/usr/bin:/bin:/usr/sbin:/sbin" \
    LANG="en_US.UTF-8" \
      TMPDIR="${TMPDIR:-/tmp}" \
      "${resources}/bin/pi" --version)" || fail "bundled Pi runtime smoke test failed"
  pi_version="$(extract_semver "${pi_version}")" || fail "unexpected bundled Pi version output: ${pi_version}"
  [[ "${pi_version}" == "${EXPECTED_PI_VERSION}" ]] || fail "bundled Pi must be ${EXPECTED_PI_VERSION}, got ${pi_version}"
  bun_version="$(extract_semver "$("${resources}/bin/bun" --version)")" || fail "bundled Bun did not report a semantic version"
  [[ "${bun_version}" == "${EXPECTED_BUN_VERSION}" ]] || fail "bundled Bun must be ${EXPECTED_BUN_VERSION}, got ${bun_version}"
  node_version="$("${resources}/bin/node" -p 'process.versions.node')" || fail "bundled Node did not report a version"
  [[ "${node_version}" == "${EXPECTED_RUNTIME_NODE_VERSION}" ]] || fail "bundled Node must be ${EXPECTED_RUNTIME_NODE_VERSION}, got ${node_version}"
  verify_auth_import_contract "${resources}"
  verify_runtime_evidence "${resources}"

  verify_fuses "${app_bundle}"
  /usr/bin/codesign --verify --deep --strict --verbose=2 "${app_bundle}"
  signature_details="$(/usr/bin/codesign -d --verbose=4 "${app_bundle}" 2>&1)"
  if [[ "${SIGNING_MODE}" == developer-id ]]; then
    [[ "${signature_details}" == *"Authority=${SIGN_IDENTITY}"* ]] || fail "bundle is not signed by the requested Developer ID identity"
    [[ "${signature_details}" == *runtime* ]] || fail "Developer ID signature is missing hardened runtime"
  else
    [[ "${signature_details}" == *'Signature=adhoc'* ]] || fail "local package must use an ad-hoc signature"
  fi
  /usr/bin/codesign -d --entitlements :- "${app_bundle}" >"${entitlements_plist}" 2>/dev/null
  /usr/bin/plutil -lint "${entitlements_plist}" >/dev/null
  assert_plist_value "${entitlements_plist}" com.apple.security.cs.allow-jit true
  if [[ "${SIGNING_MODE}" == developer-id ]]; then
    if /usr/libexec/PlistBuddy -c 'Print :com.apple.security.cs.disable-library-validation' "${entitlements_plist}" >/dev/null 2>&1; then
      fail "Developer ID build unexpectedly disables library validation"
    fi
  else
    assert_plist_value "${entitlements_plist}" com.apple.security.cs.disable-library-validation true
  fi
  for forbidden_entitlement in \
    com.apple.security.device.audio-input \
    com.apple.security.device.bluetooth \
    com.apple.security.device.camera \
    com.apple.security.device.print \
    com.apple.security.device.usb \
    com.apple.security.personal-information.location; do
    if /usr/libexec/PlistBuddy -c "Print :${forbidden_entitlement}" "${entitlements_plist}" >/dev/null 2>&1; then
      fail "unexpected top-level entitlement: ${forbidden_entitlement}"
    fi
  done
  for helper in \
    'PAD Desktop Helper.app' \
    'PAD Desktop Helper (GPU).app' \
    'PAD Desktop Helper (Renderer).app'; do
    /usr/bin/codesign -d --entitlements :- "${app_bundle}/Contents/Frameworks/${helper}" >"${entitlements_plist}" 2>/dev/null
    assert_plist_value "${entitlements_plist}" com.apple.security.cs.allow-jit true
    if [[ "${SIGNING_MODE}" == developer-id ]]; then
      if /usr/libexec/PlistBuddy -c 'Print :com.apple.security.cs.disable-library-validation' "${entitlements_plist}" >/dev/null 2>&1; then
        fail "Developer ID helper unexpectedly disables library validation: ${helper}"
      fi
    else
      assert_plist_value "${entitlements_plist}" com.apple.security.cs.disable-library-validation true
    fi
  done
  /usr/bin/codesign -d --entitlements :- "${app_bundle}/Contents/Frameworks/PAD Desktop Helper (Plugin).app" >"${entitlements_plist}" 2>/dev/null
  assert_plist_value "${entitlements_plist}" com.apple.security.cs.allow-unsigned-executable-memory true
  if [[ "${SIGNING_MODE}" == developer-id ]]; then
    if /usr/libexec/PlistBuddy -c 'Print :com.apple.security.cs.disable-library-validation' "${entitlements_plist}" >/dev/null 2>&1; then
      fail "Developer ID plugin helper unexpectedly disables library validation"
    fi
  else
    assert_plist_value "${entitlements_plist}" com.apple.security.cs.disable-library-validation true
  fi
}

APP_VERSION="$("${PACKAGE_NODE}" -p "require('${APP_DIR}/package.json').version")"
[[ "${APP_VERSION}" == <->.<->.<->* ]] || fail "invalid PAD Desktop package version: ${APP_VERSION}"

PI_PACKAGE_SOURCE="$(real_path "${PAD_PI_PACKAGE:-/opt/homebrew/lib/node_modules/@earendil-works/pi-coding-agent}")"
BUN_SOURCE="$(real_path "${PAD_BUN_BIN:-/opt/homebrew/bin/bun}")"
[[ -d "${PI_PACKAGE_SOURCE}" ]] || fail "bundled Pi package source is missing: ${PI_PACKAGE_SOURCE}"
[[ -x "${BUN_SOURCE}" ]] || fail "bundled Bun source is missing: ${BUN_SOURCE}"
assert_arm64_only "${BUN_SOURCE}"
assert_system_linkage "${BUN_SOURCE}"
[[ -f "${PI_PACKAGE_SOURCE}/package.json" ]] || fail "Pi package metadata is missing"
[[ -f "${PI_PACKAGE_SOURCE}/dist/bun/cli.js" ]] || fail "Pi Bun entrypoint is missing"
[[ -f "${PI_PACKAGE_SOURCE}/dist/bundle/cli.js" ]] || fail "Pi Node entrypoint is missing"
assert_internal_symlinks "${PI_PACKAGE_SOURCE}"
PI_SOURCE_VERSION="$("${PACKAGE_NODE}" -p "require(process.argv[1]).version" "${PI_PACKAGE_SOURCE}/package.json")"
[[ "${PI_SOURCE_VERSION}" == "${EXPECTED_PI_VERSION}" ]] || fail "Pi source must be pinned to ${EXPECTED_PI_VERSION}, got ${PI_SOURCE_VERSION}"
BUN_SOURCE_VERSION="$(extract_semver "$("${BUN_SOURCE}" --version)")" || fail "Bun source did not report a semantic version"
[[ "${BUN_SOURCE_VERSION}" == "${EXPECTED_BUN_VERSION}" ]] || fail "Bun source must be pinned to ${EXPECTED_BUN_VERSION}, got ${BUN_SOURCE_VERSION}"

/usr/bin/ditto --noqtn "${PI_PACKAGE_SOURCE}" "${RESOURCE_STAGE}/pi"
/bin/cp -p "${BUN_SOURCE}" "${RESOURCE_STAGE}/bin/bun"
/bin/cp -p "${RUNTIME_NODE}" "${RESOURCE_STAGE}/bin/node"
/bin/cp -p "${APP_DIR}/Resources/pi" "${RESOURCE_STAGE}/bin/pi"
chmod 755 "${RESOURCE_STAGE}/bin/bun" "${RESOURCE_STAGE}/bin/node" "${RESOURCE_STAGE}/bin/pi"
/usr/bin/xattr -cr "${RESOURCE_STAGE}" 2>/dev/null || true
assert_internal_symlinks "${RESOURCE_STAGE}"

STAGED_PI_VERSION="$(extract_semver "$(/usr/bin/env -i HOME="${SMOKE_HOME}" PATH="/usr/bin:/bin:/usr/sbin:/sbin" LANG="en_US.UTF-8" "${RESOURCE_STAGE}/bin/pi" --version)")" || fail "bundled Pi smoke test failed"
[[ "${STAGED_PI_VERSION}" == "${EXPECTED_PI_VERSION}" ]] || fail "bundled Pi must be ${EXPECTED_PI_VERSION}, got ${STAGED_PI_VERSION}"
STAGED_BUN_VERSION="$(extract_semver "$("${RESOURCE_STAGE}/bin/bun" --version)")" || fail "bundled Bun smoke test failed"
[[ "${STAGED_BUN_VERSION}" == "${EXPECTED_BUN_VERSION}" ]] || fail "bundled Bun must be ${EXPECTED_BUN_VERSION}, got ${STAGED_BUN_VERSION}"
STAGED_NODE_VERSION="$("${RESOURCE_STAGE}/bin/node" -p 'process.versions.node')" || fail "bundled Node smoke test failed"
[[ "${STAGED_NODE_VERSION}" == "${EXPECTED_RUNTIME_NODE_VERSION}" ]] || fail "bundled Node must be ${EXPECTED_RUNTIME_NODE_VERSION}, got ${STAGED_NODE_VERSION}"

ELECTRON_VERSION="$("${PACKAGE_NODE}" -p "require('${APP_DIR}/node_modules/electron/package.json').version")"
ELECTRON_ZIP_NAME="electron-v${ELECTRON_VERSION}-darwin-arm64.zip"
ELECTRON_CHECKSUMS="${APP_DIR}/node_modules/electron/checksums.json"
[[ -f "${ELECTRON_CHECKSUMS}" ]] || fail "Electron checksum manifest is missing"
EXPECTED_ELECTRON_SHA="$("${PACKAGE_NODE}" -e 'const c=require(process.argv[1]); process.stdout.write(c[process.argv[2]] || "")' "${ELECTRON_CHECKSUMS}" "${ELECTRON_ZIP_NAME}")"
[[ ${#EXPECTED_ELECTRON_SHA} -eq 64 && "${EXPECTED_ELECTRON_SHA}" != *[^0-9a-f]* ]] || fail "Electron checksum is missing for ${ELECTRON_ZIP_NAME}"

ELECTRON_ZIP="${PAD_ELECTRON_ZIP:-}"
if [[ -z "${ELECTRON_ZIP}" ]]; then
  ELECTRON_CACHE_ROOT="${ELECTRON_CACHE:-${HOME}/Library/Caches/electron}"
  if [[ -d "${ELECTRON_CACHE_ROOT}" ]]; then
    while IFS= read -r candidate; do
      ELECTRON_ZIP="${candidate}"
      break
    done < <(/usr/bin/find "${ELECTRON_CACHE_ROOT}" -type f -name "${ELECTRON_ZIP_NAME}" -print)
  fi
fi
[[ -n "${ELECTRON_ZIP}" && -f "${ELECTRON_ZIP}" ]] || fail "cached ${ELECTRON_ZIP_NAME} not found; release packaging will not download it"
ELECTRON_ZIP="$(real_path "${ELECTRON_ZIP}")"
ACTUAL_ELECTRON_SHA="$(/usr/bin/shasum -a 256 "${ELECTRON_ZIP}" | /usr/bin/awk '{print $1}')"
[[ "${ACTUAL_ELECTRON_SHA}" == "${EXPECTED_ELECTRON_SHA}" ]] || fail "Electron ZIP checksum mismatch"
if ! /bin/ln "${ELECTRON_ZIP}" "${ELECTRON_ZIP_STAGE}/${ELECTRON_ZIP_NAME}" 2>/dev/null; then
  /bin/cp -p "${ELECTRON_ZIP}" "${ELECTRON_ZIP_STAGE}/${ELECTRON_ZIP_NAME}"
fi

generate_runtime_evidence \
  "${APP_VERSION}" \
  "${ELECTRON_VERSION}" \
  "${ACTUAL_ELECTRON_SHA}"
verify_runtime_evidence "${RESOURCE_STAGE}"

cd "${APP_DIR}"
export PAD_ELECTRON_RESOURCE_DIR="${RESOURCE_STAGE}"
export PAD_ELECTRON_ZIP_DIR="${ELECTRON_ZIP_STAGE}"
export npm_config_offline=true
export npm_config_update_notifier=false
export npm_config_audit=false
"${SCRIPT_DIR}/run-electron-forge.sh" package --platform=darwin --arch=arm64

APP_BUNDLE="${APP_DIR}/out/PAD Desktop-darwin-arm64/PAD Desktop.app"
refresh_packaged_runtime_evidence "${APP_BUNDLE}"
verify_app_bundle "${APP_BUNDLE}"

echo "Created and verified PAD Desktop app: ${APP_BUNDLE}"
echo "Pinned runtime evidence: Pi ${EXPECTED_PI_VERSION}, Node ${EXPECTED_RUNTIME_NODE_VERSION}, Bun ${EXPECTED_BUN_VERSION}, ${SIGNING_MODE}"
echo "Desktop backend: Electron/TypeScript (no Rust sidecar)."
