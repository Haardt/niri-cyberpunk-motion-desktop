#!/usr/bin/env python3
"""Compare direct Rust dependencies against the current stable crates.io releases."""
import concurrent.futures
import datetime
import json
from pathlib import Path
import tomllib
import urllib.request

root = Path(__file__).resolve().parents[2]
projects = {}
for project in ['motion-login', 'motion-render', 'niri-motion-background']:
    manifest = tomllib.loads((root / project / 'Cargo.toml').read_text())
    lock = tomllib.loads((root / project / 'Cargo.lock').read_text())
    deps = {}
    for name, spec in (manifest.get('dependencies', {}) | manifest.get('dev-dependencies', {})).items():
        if isinstance(spec, dict) and 'path' in spec:
            continue
        version = spec if isinstance(spec, str) else spec['version']
        prefix = version.split('.')
        resolved = [p['version'] for p in lock['package'] if p['name'] == name
            and p.get('source', '').startswith('registry+')
            and (p['version'].split('.')[:2] == prefix[:2] if prefix[0] == '0' and len(prefix) > 1
                 else p['version'].split('.')[0] == prefix[0])]
        deps[name] = {'requested': version, 'locked': resolved}
    projects[project] = deps

def latest(name):
    request = urllib.request.Request('https://crates.io/api/v1/crates/' + name,
        headers={'User-Agent': 'motion-login-dependency-review/0.2'})
    with urllib.request.urlopen(request, timeout=20) as response:
        return name, json.load(response)['crate']['max_stable_version']

names = set().union(*(set(p) for p in projects.values()))
with concurrent.futures.ThreadPoolExecutor(max_workers=5) as pool:
    stable = dict(pool.map(latest, sorted(names)))
outdated = []
for project, dependencies in projects.items():
    for name, info in dependencies.items():
        info['latest_stable'] = stable[name]
        if stable[name] not in info['locked']:
            outdated.append(f'{project}: {name} {info["locked"]} -> {stable[name]}')
report = {'checked_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
          'projects': projects, 'outdated_direct_dependencies': outdated}
output = root / 'motion-login/dependency-audit.json'
output.write_text(json.dumps(report, indent=2) + '\n')
print('\n'.join(outdated) if outdated else 'All direct dependencies in the three projects match current stable releases')
