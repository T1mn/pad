#!/usr/bin/python3
"""Recover failed Zig downloads using curl, retaining the upstream URLs/hashes."""
import argparse
from pathlib import Path
import re
import subprocess
import tempfile
from upstream import ROOT, SOURCE, check, run, tool


def locked_dependencies():
    manifests = [SOURCE / path for path in run('git', 'ls-files', '*build.zig.zon', cwd=SOURCE).splitlines()]
    dependencies = {}
    visited = set()
    cache = package_cache()
    while manifests:
        path = manifests.pop()
        if path in visited or not path.is_file():
            continue
        visited.add(path)
        for url, digest in re.findall(r'\.url\s*=\s*"([^"]+)"\s*,\s*\.hash\s*=\s*"([^"]+)"', path.read_text()):
            dependencies[url] = digest
            manifests.append(cache / digest / 'build.zig.zon')
    return dependencies


def package_cache():
    env = run(tool('zig'), 'env')
    match = re.search(r'\.global_cache_dir\s*=\s*"([^"]+)"', env)
    if not match:
        raise RuntimeError('Cannot determine Zig package cache')
    return Path(match.group(1)) / 'p'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('log', type=Path, help='Build/configuration error log listing failed .url fields')
    args = parser.parse_args()
    check()
    locked = locked_dependencies()
    urls = list(dict.fromkeys(re.findall(r'\.url\s*=\s*"([^"]+)"', args.log.read_text())))
    if not urls or any(url not in locked for url in urls):
        parser.error('Log must contain only dependency URLs from the pinned source manifests')
    cache = package_cache()
    for url in urls:
        digest = locked[url]
        if (cache / digest).is_dir():
            print(f'Cached: {digest}', flush=True)
            continue
        with tempfile.TemporaryDirectory(prefix='dependency-', dir=ROOT / '.cache') as temporary:
            # A pinned GitHub git dependency has an equivalent commit archive.
            git = re.fullmatch(r'git\+https://github.com/([^/#]+/[^/#]+)#([0-9a-f]{40})', url)
            download = f'https://github.com/{git.group(1)}/archive/{git.group(2)}.tar.gz' if git else url
            if not download.startswith('https://'):
                raise RuntimeError(f'Unsupported dependency URL: {url}')
            archive = Path(temporary) / download.rsplit('/', 1)[-1]
            subprocess.run(['curl', '-fsSL', '--retry', '2', '--retry-all-errors', '--connect-timeout', '10', '--max-time', '180', download, '-o', str(archive)], check=True)
            actual = run(tool('zig'), 'fetch', archive).strip()
            if actual != digest:
                raise RuntimeError(f'Dependency checksum mismatch for {url}: expected {digest}, received {actual}')
            print(f'Verified: {digest}', flush=True)


if __name__ == '__main__':
    main()
