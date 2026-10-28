#!/usr/bin/env python3
"""Collect sibling Rust sources and shader assets into a self-contained source archive."""
from pathlib import Path
import hashlib
import tarfile
root = Path(__file__).resolve().parents[2]
output = root / 'dist/arch'
output.mkdir(parents=True, exist_ok=True)
archive = output / 'motion-login-0.3.0.tar.gz'
def include(info):
    parts = Path(info.name).parts
    return None if any(p in {'target', '.git', 'dist', '__pycache__', '.test-deps'} for p in parts) else info
with tarfile.open(archive, 'w:gz') as tar:
    for name in ['motion-login', 'motion-render']:
        tar.add(root.parent / name, arcname=f'motion-login-0.3.0/{name}', filter=include)
    for name in ['shaders', 'third_party', 'THIRD_PARTY.md', 'LICENSE']:
        tar.add(root.parent / 'niri-motion-background' / name, arcname=f'motion-login-0.3.0/niri-motion-background/{name}', filter=include)
digest = hashlib.file_digest(archive.open('rb'), 'sha256').hexdigest()
(output / 'PKGBUILD').write_text((root / 'packaging/arch/PKGBUILD').read_text().replace('SOURCE_HASH', digest))
print(f'Source snapshot: {archive}\nSHA256: {digest}')
