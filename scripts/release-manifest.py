import json, os
from pathlib import Path
root = Path('artifacts')
version = json.loads(Path('tauri.conf.json').read_text())['version']
platforms = {}
for extension, platform in [('exe', 'windows-x86_64-nsis'), ('AppImage', 'linux-x86_64-appimage'), ('deb', 'linux-x86_64-deb'), ('rpm', 'linux-x86_64-rpm')]:
    signatures = list(root.rglob('*.' + extension + '.sig'))
    assert len(signatures) == 1, f'Expected one {extension} signature: {signatures}'
    signature = signatures[0]
    name = signature.name[:-4]
    assert signature.with_name(name).is_file(), name
    platforms[platform] = {'signature': signature.read_text().strip(), 'url': f'https://github.com/MDHaarith/RoadMap-releases/releases/download/v{version}/{name}'}
# Unbundled/development copies can discover the release and offer its web page.
platforms['linux-x86_64'] = platforms['linux-x86_64-appimage']
platforms['windows-x86_64'] = platforms['windows-x86_64-nsis']
Path('latest.json').write_text(json.dumps({'version': version, 'notes': 'Roadmap update', 'platforms': platforms}, indent=2))
