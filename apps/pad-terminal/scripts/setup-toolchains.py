#!/usr/bin/python3
"""Download only the locked arm64 developer tools, without Homebrew changes."""
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import tarfile
import tempfile

root = Path(__file__).resolve().parents[1]
locked = json.loads((root / 'upstream.lock.json').read_text())
if platform.system() != 'Darwin' or platform.machine() != 'arm64':
    raise SystemExit('This preview bootstrap currently supports Apple Silicon macOS only')
cache = root / '.cache' / 'toolchains'
cache.mkdir(parents=True, exist_ok=True)
for name, spec in locked['toolchains_arm64'].items():
    target = cache / spec['directory']
    if target.exists():
        version = subprocess.check_output([str(target / name), '--version' if name == 'nu' else 'version'], text=True).strip()
        expected = locked['nu_version' if name == 'nu' else 'zig_version']
        if version != expected:
            raise SystemExit(f'{target}: unexpected version {version}; refusing to overwrite')
        print(f'{name} {version}: already available')
        continue
    with tempfile.TemporaryDirectory(prefix=f'{name}-download-', dir=cache) as temporary:
        temporary = Path(temporary)
        archive = temporary / 'tool.tar'
        subprocess.run(['curl', '-fL', '--retry', '2', '--connect-timeout', '10', '--max-time', '180', spec['url'], '-o', str(archive)], check=True)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        if digest != spec['sha256']:
            raise SystemExit(f'{name}: archive checksum mismatch')
        with tarfile.open(archive) as contents:
            # Even verified archives must stay within this temporary directory.
            for member in contents.getmembers():
                relative = Path(member.name)
                if relative.is_absolute() or '..' in relative.parts or member.issym() or member.islnk() or member.isdev():
                    raise SystemExit(f'{name}: unsupported archive member {member.name}')
            contents.extractall(temporary)
        (temporary / spec['directory']).rename(target)
        print(f'{name}: installed locally at {target}')
print('Metal component, if missing: xcodebuild -downloadComponent MetalToolchain')
