#!/usr/bin/python3
"""Pinned Ghostty preview tooling. No installs, toolchain downloads or releases."""
import argparse
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / ".cache" / "ghostty"
PATCH = ROOT / "patches" / "0001-pad-preview.patch"
NATIVE_TARGET = Path("macos/Sources/Features/PAD")


def native_files():
    return {NATIVE_TARGET / file.name: file for file in sorted((ROOT / "native").glob("*.swift"))}


def sync_native():
    """Copy owned product code; leave the pinned terminal implementation untouched."""
    for relative, source in native_files().items():
        target = SOURCE / relative
        if target.is_symlink() or target.parent.is_symlink():
            raise RuntimeError(f"Refusing a symlink in native overlay: {target}")
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists() or target.read_bytes() != source.read_bytes():
            shutil.copyfile(source, target)


def bundle_host(app):
    shutil.copyfile(ROOT / "native/PADDefaults.ghostty", app / "Contents/Resources/PADDefaults.ghostty")
    target = app / "Contents/Resources/PADHost"
    for source in (ROOT / "host").glob("*.mjs"):
        if source.name.endswith(".test.mjs"):
            continue
        output = target / "host" / source.name
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, output)
    (target / "scripts").mkdir(parents=True, exist_ok=True)
    for name in ("pi-session.mjs", "migrate-data-root.mjs"):
        shutil.copyfile(ROOT / "scripts" / name, target / "scripts" / name)


def lock():
    value = json.loads((ROOT / "upstream.lock.json").read_text())
    if not re.fullmatch(r"[0-9a-f]{40}", value["commit"]):
        raise RuntimeError("Ghostty must be pinned to a full commit SHA")
    return value


def tool(name):
    pinned = lock()
    local = ROOT / ".cache" / "toolchains" / pinned["toolchains_arm64"][name]["directory"] / name
    return str(local) if platform.machine() == "arm64" and local.is_file() else name


def run(*args, cwd=ROOT, capture=True, env=None):
    result = subprocess.run(
        [str(arg) for arg in args], cwd=cwd, check=True, text=True, env=env,
        stdout=subprocess.PIPE if capture else None,
    )
    return result.stdout if capture else ""


def verify_source(source=SOURCE):
    pinned = lock()
    if not (source / ".git").is_dir() or source.is_symlink():
        raise RuntimeError(f"Not an owned Ghostty checkout: {source}")
    if run("git", "rev-parse", "HEAD", cwd=source).strip() != pinned["commit"]:
        raise RuntimeError("Ghostty checkout differs from upstream.lock.json; refusing to reset it")
    if run("git", "remote", "get-url", "origin", cwd=source).strip() != pinned["repository"]:
        raise RuntimeError("Unexpected Ghostty remote")
    zon = (source / "build.zig.zon").read_text()
    for field, expected in (("version", pinned["tag"].removeprefix("v")), ("minimum_zig_version", pinned["zig_version"])):
        found = re.search(rf'\.{field}\s*=\s*"([^"]+)"', zon)
        if not found or found.group(1) != expected:
            raise RuntimeError(f"Upstream {field} differs from the lock")


def fetch():
    if SOURCE.exists():
        verify_source()
        print(f"Using pinned checkout: {SOURCE}")
        return
    pinned = lock()
    SOURCE.parent.mkdir(parents=True, exist_ok=True)
    # Publish only a verified checkout; interrupted downloads do not look ready.
    with tempfile.TemporaryDirectory(prefix="ghostty-fetch-", dir=SOURCE.parent) as temporary:
        checkout = Path(temporary) / "source"
        run("git", "init", checkout, capture=False)
        run("git", "remote", "add", "origin", pinned["repository"], cwd=checkout)
        run("git", "fetch", "--depth=1", "--no-tags", "origin", pinned["commit"], cwd=checkout, capture=False)
        run("git", "checkout", "--detach", "FETCH_HEAD", cwd=checkout, capture=False)
        verify_source(checkout)
        checkout.rename(SOURCE)
    print(f"Fetched {pinned['tag']} at {pinned['commit']}")


def source_diff():
    verify_source()
    if run("git", "diff", "--cached", "--name-only", cwd=SOURCE).strip():
        raise RuntimeError("Upstream checkout has staged changes; refusing to overwrite")
    untracked = set(filter(None, run("git", "ls-files", "--others", "--exclude-standard", "-z", cwd=SOURCE).split("\0")))
    if untracked - {str(relative) for relative in native_files()}:
        raise RuntimeError("Upstream checkout has unknown untracked files; refusing to overwrite")
    return run("git", "diff", "--binary", "--full-index", "--no-ext-diff", cwd=SOURCE)


def prepare():
    fetch()
    expected = PATCH.read_text()
    actual = source_diff()
    if actual != expected:
        if actual:
            raise RuntimeError("Upstream checkout has local changes; save them as reviewed patches first")
        run("git", "apply", "--check", PATCH, cwd=SOURCE)
        run("git", "apply", PATCH, cwd=SOURCE)
    sync_native()
    check()


def check():
    if source_diff() != PATCH.read_text():
        raise RuntimeError("Checkout must contain exactly the reviewed PAD patch; run prepare")
    for relative, source in native_files().items():
        target = SOURCE / relative
        if not target.is_file() or target.is_symlink() or target.read_bytes() != source.read_bytes():
            raise RuntimeError(f"Native overlay differs: {relative}; run prepare")
    print("Pinned source, PAD patch and owned native overlay match")


def doctor():
    pinned = lock()
    failures = []
    if platform.system() != "Darwin":
        failures.append("The preview requires macOS")
    for executable in ("git", tool("zig"), tool("nu"), "xcodebuild", "xcrun"):
        if not shutil.which(executable):
            failures.append(f"Missing {executable}")
    if shutil.which(tool("nu")):
        version = run(tool("nu"), "--version").strip()
        print(f"Nushell: {version}")
        if version != pinned["nu_version"]:
            failures.append(f"Use Nushell {pinned['nu_version']}")
    if shutil.which(tool("zig")):
        version = run(tool("zig"), "version").strip()
        print(f"Zig: {version} (required {pinned['zig_version']})")
        if version != pinned["zig_version"]:
            failures.append(f"Use Zig {pinned['zig_version']}, not the latest Zig")
    if shutil.which("xcodebuild"):
        version = run("xcodebuild", "-version").strip()
        print(version)
        match = re.search(r"Xcode (\d+)", version)
        if not match or int(match.group(1)) < pinned["xcode_major"]:
            failures.append(f"Xcode {pinned['xcode_major']} or newer is required")
    if shutil.which("xcrun"):
        run("xcrun", "--sdk", "macosx", "--show-sdk-version", capture=False)
        run("xcrun", "metal", "--version", capture=False)
    free = shutil.disk_usage(ROOT).free / 1024**3
    print(f"Free disk: {free:.1f} GiB; initial compilation may consume several GiB")
    if free < 10:
        failures.append("Keep at least 10 GiB free before building (local safety reserve)")
    if failures:
        raise RuntimeError("; ".join(failures))
    print("Build prerequisites found; this does not validate a compiled app")


def build(native_only=False, output_dir=None):
    directory = (ROOT / (output_dir or "out")).resolve()
    if not directory.is_relative_to(ROOT):
        raise RuntimeError("Preview output must stay inside apps/pad-terminal")
    output = directory / f"{lock()['preview_name']}.app"
    if output.exists():
        raise RuntimeError(f"Preview already exists; move it aside before rebuilding: {output}")
    doctor()
    check()
    # Keep SDK compatibility local to Zig's macOS discovery, never xcode-select.
    zig_env = dict(os.environ)
    if zig_env.get("PAD_ZIG_MACOS_SDK"):
        sdk = Path(zig_env["PAD_ZIG_MACOS_SDK"]).resolve()
        if not (sdk / "usr/lib/libSystem.tbd").is_file():
            raise RuntimeError("PAD_ZIG_MACOS_SDK must point to an installed macOS SDK")
        zig_env["PAD_ZIG_MACOS_SDK"] = str(sdk)
        zig_env["PATH"] = str(ROOT / "scripts/zig-sdk") + os.pathsep + zig_env.get("PATH", os.defpath)
        print(f"Zig macOS SDK: {sdk}; Xcode/Metal still use the selected Xcode")
    if zig_env.get("PAD_ZIG_LIBTOOL"):
        archiver = Path(zig_env["PAD_ZIG_LIBTOOL"]).resolve()
        if not archiver.is_file() or not os.access(archiver, os.X_OK):
            raise RuntimeError("PAD_ZIG_LIBTOOL must point to an executable Apple libtool")
        zig_env["PAD_ZIG_LIBTOOL"] = str(archiver)
        print(f"Zig archive tool: {archiver}")
    # Native architecture only, bounded parallelism, no test/benchmark targets.
    if not native_only:
        run(tool("zig"), "build", "-Doptimize=ReleaseFast", "-Dxcframework-target=native",
            "-Demit-docs=false", "-Demit-macos-app=false", f"-Dversion-string={lock()['tag'][1:]}", "-j2",
            cwd=SOURCE, capture=False, env=zig_env)
    else:
        print("Native-only rebuild: reuse the existing GhosttyKit (no Zig/core changes allowed)")
    run(tool("nu"), "macos/build.nu", "--configuration", "ReleaseLocal", cwd=SOURCE, capture=False)
    app = SOURCE / "macos" / "build" / "ReleaseLocal" / "Ghostty.app"
    with (app / "Contents" / "Info.plist").open("rb") as stream:
        info = plistlib.load(stream)
    if info.get("CFBundleIdentifier") != lock()["preview_bundle_id"]:
        raise RuntimeError("Built app has the upstream identity; refusing to publish preview")
    from notices import bundle_notices
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".preview-stage-", dir=output.parent) as temporary:
        staged = Path(temporary) / output.name
        shutil.copytree(app, staged, symlinks=True)
        bundle_notices(staged)
        bundle_host(staged)
        # Adding notices changes the resource seal; preserve native entitlements.
        run("codesign", "--force", "--sign", "-", "--preserve-metadata=entitlements,flags,runtime", staged, capture=False)
        run("codesign", "--verify", "--deep", "--strict", staged, capture=False)
        staged.rename(output)
    print(f"Built locally: {output}\nNot installed, launched, notarized or published.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("doctor", "fetch", "prepare", "check", "build"))
    parser.add_argument("--native-only", action="store_true", help="build: reuse previously built GhosttyKit for Swift-only changes")
    parser.add_argument("--output-dir", help="build: separate output directory under apps/pad-terminal; never overwrite a running preview")
    args = parser.parse_args()
    try:
        if (args.native_only or args.output_dir) and args.action != "build":
            raise RuntimeError("--native-only/--output-dir apply only to build")
        if args.action == "build":
            build(native_only=args.native_only, output_dir=args.output_dir)
        else:
            globals()[args.action]()
    except (RuntimeError, OSError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"ERROR: {error}\n")


if __name__ == "__main__":
    main()
