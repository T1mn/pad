"""Keep upstream and dependency license texts inside the local preview bundle."""
from pathlib import Path
import re
import shutil
from dependencies import locked_dependencies, package_cache
from upstream import SOURCE, lock, run

LICENSE_NAME = re.compile(r'^(licen[sc]e|copying|copyright|notice|ofl|ftl|gpl|lgpl|apache)([._-]|$)', re.I)


def bundle_notices(app):
    destination = app / 'Contents/Resources/PAD-Licenses'
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(SOURCE / 'LICENSE', destination / 'Ghostty-MIT.txt')
    manifest = [f"PAD Terminal Preview is a modified Ghostty {lock()['tag']} ({lock()['commit']}).",
                f"Source: {lock()['repository']}",
                'Local modifications: preview identity/configuration, disabled updater, native build settings.',
                'The original MIT notice is in Ghostty-MIT.txt. Dependency notices follow.',
                'The upstream name/icon remain visible in parts of this development preview.', '']
    packages = [('upstream', SOURCE)]
    cache = package_cache()
    packages.extend((digest, cache / digest) for digest in sorted(set(locked_dependencies().values())) if (cache / digest).is_dir())
    checkouts = SOURCE / 'macos/build/DerivedData/SourcePackages/checkouts'
    if checkouts.is_dir():
        packages.extend((f'swift-{path.name}', path) for path in sorted(checkouts.iterdir()) if path.is_dir())
    count = 0
    for name, root in packages:
        if root == SOURCE:
            files = [root / relative for relative in run('git', 'ls-files', cwd=SOURCE).splitlines()]
        else:
            files = [path for path in root.rglob('*') if '.git' not in path.parts and path.is_file()]
        for path in files:
            if not LICENSE_NAME.match(path.name) or not path.is_file():
                continue
            target = destination / name / path.relative_to(root)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, target)
            manifest.append(str(target.relative_to(destination)))
            count += 1
    if count < 2:
        raise RuntimeError('Dependency license collection was unexpectedly empty')
    (destination / 'README.txt').write_text('\n'.join(manifest) + '\n')
    print(f'Preserved {count} license/notice files in {destination}')
