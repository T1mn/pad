#!/bin/bash
set -euo pipefail

# Compile every PAD Remote source for both Apple target triples without relying
# on CoreSimulator being bootable. XCTest sources are checked against the
# emitted @testable module as a separate final gate.

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
ios_root="$repo_root/apps/pad-ios"
developer_dir="${DEVELOPER_DIR:-/Applications/Xcode.app/Contents/Developer}"
export DEVELOPER_DIR="$developer_dir"

simulator_sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
device_sdk="$(xcrun --sdk iphoneos --show-sdk-path)"
typecheck_root="$(mktemp -d "${TMPDIR:-/tmp}/pad-ios-typecheck.XXXXXX")"
trap 'rm -rf -- "$typecheck_root"' EXIT

cd "$ios_root"

find PADRemote -name '*.swift' -print0 | xargs -0 xcrun swiftc \
  -typecheck \
  -parse-as-library \
  -module-name PADRemote \
  -target arm64-apple-ios17.0-simulator \
  -sdk "$simulator_sdk"
echo '[PASS] PAD Remote app sources type-check for arm64 iOS Simulator'

find PADRemote -name '*.swift' -print0 | xargs -0 xcrun swiftc \
  -typecheck \
  -parse-as-library \
  -module-name PADRemote \
  -target arm64-apple-ios17.0 \
  -sdk "$device_sdk"
echo '[PASS] PAD Remote app sources type-check for arm64 iOS device'

find PADRemote -name '*.swift' -print0 | xargs -0 xcrun swiftc \
  -emit-module \
  -emit-module-path "$typecheck_root/PADRemote.swiftmodule" \
  -parse-as-library \
  -enable-testing \
  -module-name PADRemote \
  -target arm64-apple-ios17.0-simulator \
  -sdk "$simulator_sdk"

find PADRemoteTests -name '*.swift' -print0 | xargs -0 xcrun swiftc \
  -typecheck \
  -parse-as-library \
  -module-name PADRemoteTests \
  -I "$typecheck_root" \
  -I "$developer_dir/Platforms/iPhoneSimulator.platform/Developer/usr/lib" \
  -F "$developer_dir/Platforms/iPhoneSimulator.platform/Developer/Library/Frameworks" \
  -target arm64-apple-ios17.0-simulator \
  -sdk "$simulator_sdk"
echo '[PASS] PAD Remote XCTest sources type-check against the emitted app module'
