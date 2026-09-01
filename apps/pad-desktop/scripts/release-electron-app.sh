#!/bin/zsh
set -euo pipefail

SCRIPT_DIR="${0:A:h}"
APP_DIR="${SCRIPT_DIR:h}"
OUTPUT_DIR="${1:-${PAD_DESKTOP_OUTPUT_DIR:-${APP_DIR}/release}}"
RELEASE_STAGE="$(mktemp -d /tmp/pad-desktop-release.XXXXXX)"
DMG_MOUNT="${RELEASE_STAGE}/dmg-mount"
DMG_ATTACHED=0
EXPECTED_PI_VERSION="0.84.4"
EXPECTED_RUNTIME_NODE_VERSION="22.19.0"
SIGN_IDENTITY="${PAD_DESKTOP_SIGN_IDENTITY:-}"
NOTARY_PROFILE="${PAD_DESKTOP_NOTARY_PROFILE:-}"
NOTARY_KEYCHAIN="${PAD_DESKTOP_NOTARY_KEYCHAIN:-}"
SIGNING_MODE="ad-hoc-local-only"

if [[ -n "${SIGN_IDENTITY}" ]]; then
  [[ "${SIGN_IDENTITY}" == 'Developer ID Application:'* ]] || {
    echo "PAD Desktop release error: PAD_DESKTOP_SIGN_IDENTITY must name a Developer ID Application certificate" >&2
    exit 1
  }
  [[ -n "${NOTARY_PROFILE}" ]] || {
    echo "PAD Desktop release error: Developer ID release requires PAD_DESKTOP_NOTARY_PROFILE" >&2
    exit 1
  }
  SIGNING_MODE="developer-id-notarized"
elif [[ -n "${NOTARY_PROFILE}" || -n "${NOTARY_KEYCHAIN}" ]]; then
  echo "PAD Desktop release error: notarization credentials require PAD_DESKTOP_SIGN_IDENTITY" >&2
  exit 1
fi

fail() {
  echo "PAD Desktop release error: $*" >&2
  exit 1
}

submit_notarization() {
  local artifact="$1"
  local evidence="$2"
  local -a arguments

  arguments=(notarytool submit "${artifact}" --keychain-profile "${NOTARY_PROFILE}" --wait --output-format json)
  if [[ -n "${NOTARY_KEYCHAIN}" ]]; then
    [[ -f "${NOTARY_KEYCHAIN}" && ! -L "${NOTARY_KEYCHAIN}" ]] || fail "invalid notary keychain path: ${NOTARY_KEYCHAIN}"
    arguments+=(--keychain "${NOTARY_KEYCHAIN}")
  fi
  /usr/bin/xcrun "${arguments[@]}" >"${evidence}"
  /usr/bin/python3 - "${evidence}" <<'PY'
import json
import pathlib
import sys

result = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
if result.get("status") != "Accepted":
    raise SystemExit(f"Apple notarization was not accepted: {result.get('status', 'unknown')}")
if not result.get("id"):
    raise SystemExit("Apple notarization response has no submission id")
PY
}

verify_runtime_evidence() {
  local resources="$1"
  (
    cd "${resources}"
    /usr/bin/shasum -a 256 -c release-evidence/runtime-SHA256SUMS.txt >/dev/null
  ) || fail "bundled runtime checksum evidence failed verification"
  /usr/bin/python3 - \
    "${resources}/release-evidence/runtime-manifest.json" \
    "${resources}/release-evidence/runtime-sbom.spdx.json" \
    "${EXPECTED_PI_VERSION}" \
    "${EXPECTED_RUNTIME_NODE_VERSION}" <<'PY'
import json
import pathlib
import sys

manifest = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
sbom = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
pi_version, node_version = sys.argv[3:]
components = {item.get("name"): item.get("version") for item in manifest.get("components", [])}
packages = {item.get("name"): item.get("versionInfo") for item in sbom.get("packages", [])}
if manifest.get("schema") != "cn.ghostcloud.pad.desktop.runtime-evidence.v1":
    raise SystemExit("invalid runtime manifest schema")
if components.get("Pi coding agent") != pi_version or components.get("Node.js") != node_version:
    raise SystemExit("runtime manifest does not contain pinned Pi/Node.js versions")
if "Bun" in components:
    raise SystemExit("runtime manifest still claims an unbundled Bun runtime")
if packages.get("@earendil-works/pi-coding-agent") != pi_version or packages.get("Node.js") != node_version:
    raise SystemExit("SPDX SBOM does not contain pinned Pi/Node.js versions")
if "Bun" in packages:
    raise SystemExit("SPDX SBOM still claims an unbundled Bun runtime")
PY
}

verify_retained_locales() {
  /usr/bin/python3 - "$1" <<'PY'
import pathlib
import re
import sys

bundle = pathlib.Path(sys.argv[1])
roots = [
    bundle / "Contents" / "Resources",
    bundle / "Contents" / "Frameworks" / "Electron Framework.framework" / "Versions" / "A" / "Resources",
]
allowed = re.compile(r"^(?:en|zh_CN|zh_TW)[^/]*\.lproj$")
for root in roots:
    if (
        not root.is_dir()
        or root.is_symlink()
        or bundle.resolve() not in (root.resolve(), *root.resolve().parents)
    ):
        raise SystemExit(f"invalid locale root: {root}")
    unexpected = sorted(
        item.name for item in root.iterdir()
        if item.name.endswith(".lproj") and not allowed.fullmatch(item.name)
    )
    if unexpected:
        raise SystemExit(f"unexpected locales in {root}: {', '.join(unexpected)}")
PY
}

cleanup_release_stage() {
  if (( DMG_ATTACHED )); then
    /usr/bin/hdiutil detach "${DMG_MOUNT}" >/dev/null 2>&1 || true
  fi
  if [[ -n "${RELEASE_STAGE:-}" && -d "${RELEASE_STAGE}" && "${RELEASE_STAGE}" == /tmp/pad-desktop-release.* ]]; then
    rm -rf -- "${RELEASE_STAGE}"
  fi
}
trap cleanup_release_stage EXIT

OUTPUT_DIR="$(/usr/bin/python3 -c 'import os, sys; print(os.path.abspath(sys.argv[1]))' "${OUTPUT_DIR}")"
case "${OUTPUT_DIR}" in
  /|/Applications|"${HOME}"|"${APP_DIR}") fail "refusing unsafe release output directory: ${OUTPUT_DIR}" ;;
esac
if [[ -e "${OUTPUT_DIR}" && -L "${OUTPUT_DIR}" ]]; then
  fail "release output directory must not be a symlink: ${OUTPUT_DIR}"
fi
mkdir -p "${OUTPUT_DIR}"
[[ -d "${OUTPUT_DIR}" && ! -L "${OUTPUT_DIR}" ]] || fail "invalid release output directory: ${OUTPUT_DIR}"

if [[ "${SIGNING_MODE}" == developer-id-notarized ]]; then
  IDENTITY_LIST="${RELEASE_STAGE}/codesigning-identities.txt"
  /usr/bin/security find-identity -v -p codesigning >"${IDENTITY_LIST}"
  /usr/bin/grep -F -- "\"${SIGN_IDENTITY}\"" "${IDENTITY_LIST}" >/dev/null || fail "requested Developer ID identity is not available in the keychain"
  /usr/bin/xcrun --find notarytool >/dev/null || fail "xcrun notarytool is unavailable"
  /usr/bin/xcrun --find stapler >/dev/null || fail "xcrun stapler is unavailable"
fi

"${SCRIPT_DIR}/package-electron-app.sh"

APP_BUNDLE="${APP_DIR}/out/PAD Desktop-darwin-arm64/PAD Desktop.app"
INFO_PLIST="${APP_BUNDLE}/Contents/Info.plist"
RESOURCES="${APP_BUNDLE}/Contents/Resources"
if [[ ! -d "${APP_BUNDLE}" || -L "${APP_BUNDLE}" || ! -f "${INFO_PLIST}" ]]; then
  fail "final PAD Desktop bundle not found: ${APP_BUNDLE}"
fi
/usr/bin/codesign --verify --deep --strict --verbose=2 "${APP_BUNDLE}"
[[ ! -e "${RESOURCES}/bin/bun" ]] || fail "release bundle unexpectedly contains Bun"
for required in \
  "${RESOURCES}/app.asar" \
  "${RESOURCES}/bin/node" \
  "${RESOURCES}/bin/pi" \
  "${RESOURCES}/pi/package.json" \
  "${RESOURCES}/pi/dist/bundle/cli.js" \
  "${RESOURCES}/release-evidence/runtime-manifest.json" \
  "${RESOURCES}/release-evidence/runtime-sbom.spdx.json" \
  "${RESOURCES}/release-evidence/runtime-SHA256SUMS.txt"; do
  [[ -f "${required}" ]] || fail "release resource is missing: ${required}"
done
verify_runtime_evidence "${RESOURCES}"
verify_retained_locales "${APP_BUNDLE}"

APP_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "${INFO_PLIST}")"
[[ "${APP_VERSION}" == <->.<->.<->* ]] || fail "invalid app version: ${APP_VERSION}"
ARCHIVE_BASE="PAD-Desktop-${APP_VERSION}-arm64"
ZIP_PATH="${RELEASE_STAGE}/${ARCHIVE_BASE}.zip"
DMG_PATH="${RELEASE_STAGE}/${ARCHIVE_BASE}.dmg"
MANIFEST_NAME="${ARCHIVE_BASE}-SHA256SUMS.txt"
MANIFEST_PATH="${RELEASE_STAGE}/${MANIFEST_NAME}"
DMG_STAGE="${RELEASE_STAGE}/dmg-root"
ZIP_VERIFY="${RELEASE_STAGE}/zip-verify"
RUNTIME_MANIFEST_NAME="${ARCHIVE_BASE}-runtime-manifest.json"
RUNTIME_SBOM_NAME="${ARCHIVE_BASE}-runtime-sbom.spdx.json"
RUNTIME_CHECKSUMS_NAME="${ARCHIVE_BASE}-runtime-SHA256SUMS.txt"
RELEASE_EVIDENCE_NAME="${ARCHIVE_BASE}-release-evidence.json"
APP_NOTARY_EVIDENCE_NAME="${ARCHIVE_BASE}-app-notarization.json"
DMG_NOTARY_EVIDENCE_NAME="${ARCHIVE_BASE}-dmg-notarization.json"

ARTIFACT_NAMES=(
  "${ARCHIVE_BASE}.zip"
  "${ARCHIVE_BASE}.dmg"
  "${MANIFEST_NAME}"
  "${RUNTIME_MANIFEST_NAME}"
  "${RUNTIME_SBOM_NAME}"
  "${RUNTIME_CHECKSUMS_NAME}"
  "${RELEASE_EVIDENCE_NAME}"
  SHA256SUMS.txt
)
if [[ "${SIGNING_MODE}" == developer-id-notarized ]]; then
  ARTIFACT_NAMES+=("${APP_NOTARY_EVIDENCE_NAME}" "${DMG_NOTARY_EVIDENCE_NAME}")
fi
for name in "${ARTIFACT_NAMES[@]}"; do
  if [[ -e "${OUTPUT_DIR}/${name}" && "${PAD_DESKTOP_RELEASE_REPLACE:-0}" != 1 ]]; then
    fail "release artifact already exists: ${OUTPUT_DIR}/${name} (set PAD_DESKTOP_RELEASE_REPLACE=1 to replace exact artifact names)"
  fi
done

SIGNATURE_DETAILS="$(/usr/bin/codesign -d --verbose=4 "${APP_BUNDLE}" 2>&1)"
if [[ "${SIGNING_MODE}" == developer-id-notarized ]]; then
  [[ "${SIGNATURE_DETAILS}" == *"Authority=${SIGN_IDENTITY}"* ]] || fail "packaged app does not use the requested Developer ID identity"
  [[ "${SIGNATURE_DETAILS}" == *runtime* ]] || fail "packaged app is missing hardened runtime"
  APP_NOTARY_ZIP="${RELEASE_STAGE}/${ARCHIVE_BASE}-notary-submit.zip"
  /usr/bin/ditto -c -k --sequesterRsrc --keepParent "${APP_BUNDLE}" "${APP_NOTARY_ZIP}"
  submit_notarization "${APP_NOTARY_ZIP}" "${RELEASE_STAGE}/${APP_NOTARY_EVIDENCE_NAME}"
  /usr/bin/xcrun stapler staple "${APP_BUNDLE}"
  /usr/bin/xcrun stapler validate "${APP_BUNDLE}"
  /usr/sbin/spctl --assess --type execute --verbose=4 "${APP_BUNDLE}"
  /usr/bin/codesign --verify --deep --strict --verbose=2 "${APP_BUNDLE}"
else
  [[ "${SIGNATURE_DETAILS}" == *'Signature=adhoc'* ]] || fail "local-only release is not ad-hoc signed"
fi

/usr/bin/ditto -c -k --sequesterRsrc --keepParent "${APP_BUNDLE}" "${ZIP_PATH}"
/usr/bin/unzip -tq "${ZIP_PATH}" >/dev/null
mkdir -p "${ZIP_VERIFY}"
/usr/bin/ditto -x -k "${ZIP_PATH}" "${ZIP_VERIFY}"
/usr/bin/codesign --verify --deep --strict --verbose=2 "${ZIP_VERIFY}/PAD Desktop.app"
[[ -x "${ZIP_VERIFY}/PAD Desktop.app/Contents/Resources/bin/node" ]] || fail "ZIP lost the Node.js executable bit"
verify_retained_locales "${ZIP_VERIFY}/PAD Desktop.app"

mkdir -p "${DMG_STAGE}"
/usr/bin/ditto "${APP_BUNDLE}" "${DMG_STAGE}/PAD Desktop.app"
/bin/ln -s /Applications "${DMG_STAGE}/Applications"
/usr/bin/hdiutil create \
  -volname "PAD Desktop" \
  -srcfolder "${DMG_STAGE}" \
  -format UDZO \
  -ov \
  "${DMG_PATH}" >/dev/null
if [[ "${SIGNING_MODE}" == developer-id-notarized ]]; then
  submit_notarization "${DMG_PATH}" "${RELEASE_STAGE}/${DMG_NOTARY_EVIDENCE_NAME}"
  /usr/bin/xcrun stapler staple "${DMG_PATH}"
  /usr/bin/xcrun stapler validate "${DMG_PATH}"
  /usr/sbin/spctl --assess --type open --context context:primary-signature --verbose=4 "${DMG_PATH}"
fi
/usr/bin/hdiutil verify "${DMG_PATH}" >/dev/null
mkdir -p "${DMG_MOUNT}"
/usr/bin/hdiutil attach -readonly -nobrowse -mountpoint "${DMG_MOUNT}" "${DMG_PATH}" >/dev/null
DMG_ATTACHED=1
/usr/bin/codesign --verify --deep --strict --verbose=2 "${DMG_MOUNT}/PAD Desktop.app"
verify_retained_locales "${DMG_MOUNT}/PAD Desktop.app"
/usr/bin/hdiutil detach "${DMG_MOUNT}" >/dev/null
DMG_ATTACHED=0

/bin/cp -p "${RESOURCES}/release-evidence/runtime-manifest.json" "${RELEASE_STAGE}/${RUNTIME_MANIFEST_NAME}"
/bin/cp -p "${RESOURCES}/release-evidence/runtime-sbom.spdx.json" "${RELEASE_STAGE}/${RUNTIME_SBOM_NAME}"
/bin/cp -p "${RESOURCES}/release-evidence/runtime-SHA256SUMS.txt" "${RELEASE_STAGE}/${RUNTIME_CHECKSUMS_NAME}"
TEAM_IDENTIFIER="$(/usr/bin/sed -n 's/^TeamIdentifier=//p' <<<"${SIGNATURE_DETAILS}" | /usr/bin/head -n 1)"
CDHASH="$(/usr/bin/sed -n 's/^CDHash=//p' <<<"${SIGNATURE_DETAILS}" | /usr/bin/head -n 1)"
/usr/bin/python3 - \
  "${RELEASE_STAGE}/${RELEASE_EVIDENCE_NAME}" \
  "${APP_VERSION}" \
  "${SIGNING_MODE}" \
  "${SIGN_IDENTITY}" \
  "${TEAM_IDENTIFIER}" \
  "${CDHASH}" <<'PY'
import json
import pathlib
import sys

destination, version, mode, identity, team_identifier, cdhash = sys.argv[1:]
notarized = mode == "developer-id-notarized"
team_identifier = None if team_identifier in {"", "not set"} else team_identifier
evidence = {
    "schema": "cn.ghostcloud.pad.desktop.release-evidence.v1",
    "product": "PAD Desktop",
    "version": version,
    "target": "darwin-arm64",
    "minimum_macos": "13.0",
    "signing": {
        "mode": mode,
        "identity": identity or None,
        "team_identifier": team_identifier,
        "cdhash": cdhash or None,
        "hardened_runtime": True,
    },
    "notarization": {
        "accepted": notarized,
        "app_staple_validated": notarized,
        "dmg_staple_validated": notarized,
        "gatekeeper_assessed": notarized,
    },
    "distribution_scope": "external" if notarized else "local-only",
    "runtime_evidence": {
        "manifest": "runtime-manifest.json",
        "sbom": "runtime-sbom.spdx.json",
        "checksums": "runtime-SHA256SUMS.txt",
    },
}
pathlib.Path(destination).write_text(
    json.dumps(evidence, ensure_ascii=False, indent=2) + "\n",
    encoding="utf-8",
)
PY

CHECKSUM_INPUTS=(
  "${ARCHIVE_BASE}.zip"
  "${ARCHIVE_BASE}.dmg"
  "${RUNTIME_MANIFEST_NAME}"
  "${RUNTIME_SBOM_NAME}"
  "${RUNTIME_CHECKSUMS_NAME}"
  "${RELEASE_EVIDENCE_NAME}"
)
if [[ "${SIGNING_MODE}" == developer-id-notarized ]]; then
  CHECKSUM_INPUTS+=("${APP_NOTARY_EVIDENCE_NAME}" "${DMG_NOTARY_EVIDENCE_NAME}")
fi
(
  cd "${RELEASE_STAGE}"
  /usr/bin/shasum -a 256 "${CHECKSUM_INPUTS[@]}" > "${MANIFEST_NAME}"
)

FINAL_ARTIFACTS=("${CHECKSUM_INPUTS[@]}" "${MANIFEST_NAME}")
for name in "${FINAL_ARTIFACTS[@]}"; do
  /bin/mv -f "${RELEASE_STAGE}/${name}" "${OUTPUT_DIR}/${name}"
done
/bin/cp -f "${OUTPUT_DIR}/${MANIFEST_NAME}" "${OUTPUT_DIR}/SHA256SUMS.txt"

echo "Created and verified release artifacts:"
echo "  ${OUTPUT_DIR}/${ARCHIVE_BASE}.zip"
echo "  ${OUTPUT_DIR}/${ARCHIVE_BASE}.dmg"
echo "  ${OUTPUT_DIR}/${MANIFEST_NAME}"
echo "  ${OUTPUT_DIR}/${RUNTIME_SBOM_NAME}"
echo "  ${OUTPUT_DIR}/${RELEASE_EVIDENCE_NAME}"
if [[ "${SIGNING_MODE}" == developer-id-notarized ]]; then
  echo "Developer ID signature, Apple notarization, app/DMG staple, and Gatekeeper assessment all passed."
else
  echo "LOCAL-ONLY: this build is ad-hoc signed and was not notarized or Gatekeeper-approved for external distribution."
fi
