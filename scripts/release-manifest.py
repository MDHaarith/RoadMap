import json
from pathlib import Path
# Writes <platform>.json (what the app reads, one per platform so Linux and Windows release on
# their own) for each platform in artifacts/, and latest.json (what 1.0.0 and 1.0.1 read) for the
# release, merged with old-latest.json when the other platform is already in this version.
root = Path('artifacts')
version = json.loads(Path('tauri.conf.json').read_text())['version']
base = f'https://github.com/MDHaarith/RoadMap-releases/releases/download/v{version}/'
kinds = {
    'linux': [('AppImage', 'linux-x86_64-appimage'), ('deb', 'linux-x86_64-deb'), ('rpm', 'linux-x86_64-rpm')],
    'windows': [('exe', 'windows-x86_64-nsis')],
}
# Untagged builds look up the bare key: the AppImage on Linux, and on Windows the installer
# (the -nsis name stays because 1.0.0 NSIS installs look it up; the exe is Inno Setup now).
plain = {'linux': ('linux-x86_64', 'linux-x86_64-appimage'), 'windows': ('windows-x86_64', 'windows-x86_64-nsis')}
old = Path('old-latest.json')
merged = json.loads(old.read_text())['platforms'] if old.is_file() else {}
built = []
for platform, packages in kinds.items():
    entries = {}
    for extension, key in packages:
        signatures = list(root.rglob(f'*.{extension}.sig'))
        if not signatures:
            continue
        assert len(signatures) == 1, f'Expected one {extension} signature: {signatures}'
        name = signatures[0].name[:-4]
        assert signatures[0].with_name(name).is_file(), name
        entries[key] = {'signature': signatures[0].read_text().strip(), 'url': base + name}
    if not entries:
        continue
    assert len(entries) == len(packages), f'{platform}: missing packages, found only {list(entries)}'
    alias, source = plain[platform]
    entries[alias] = entries[source]
    Path(f'{platform}.json').write_text(json.dumps({'version': version, 'notes': 'Roadmap update', 'platforms': entries}, indent=2))
    merged.update(entries)
    built.append(platform)
assert built, 'No signed packages in artifacts/'
Path('latest.json').write_text(json.dumps({'version': version, 'notes': 'Roadmap update', 'platforms': merged}, indent=2))
print('Manifests for', ', '.join(built))
