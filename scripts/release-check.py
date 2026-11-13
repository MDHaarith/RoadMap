import json, os, re, sys
from pathlib import Path
root = Path(__file__).resolve().parent.parent
cargo = re.search(r'^version = "([^"]+)"', (root / 'Cargo.toml').read_text(), re.M)[1]
versions = [cargo, json.loads((root / 'tauri.conf.json').read_text())['version'], json.loads((root / 'ui/package.json').read_text())['version']]
assert len(set(versions)) == 1, f'Versions differ: Cargo, Tauri, UI: {versions}'
spec = re.search(r'^Version:\s+(\S+)', (root / 'roadmap-app.spec').read_text(), re.M)[1]
assert spec == cargo, f'RPM spec version {spec} differs from {cargo}'
lock = re.search(r'name = "roadmap-app"\nversion = "([^"]+)"', (root / 'Cargo.lock').read_text())[1]
assert lock == cargo, f'Cargo.lock version {lock} differs from {cargo}'
ui_lock = json.loads((root / 'ui/package-lock.json').read_text())
assert ui_lock['version'] == ui_lock['packages']['']['version'] == cargo, 'Update UI lockfile root versions'
tag = os.environ.get('GITHUB_REF_NAME')
if tag:
    assert tag == 'v' + cargo, f'Release must run on tag v{cargo}, got {tag}'
print('Release versions match: ' + cargo)
