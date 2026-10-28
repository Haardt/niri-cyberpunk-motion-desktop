#!/usr/bin/env python3
"""Archive the wallpaper and sibling renderer, without build products."""
from pathlib import Path
import hashlib
import tarfile

root = Path(__file__).resolve().parents[2]
output = root / 'dist/arch'
output.mkdir(parents=True, exist_ok=True)
archive = output / 'niri-motion-background-0.1.0.tar.gz'


def include(info):
    excluded = {'target', '.git', 'dist', '__pycache__', '.test-deps'}
    return None if excluded.intersection(Path(info.name).parts) else info


with tarfile.open(archive, 'w:gz') as tar:
    for name in ['niri-motion-background', 'motion-render']:
        tar.add(root.parent / name,
                arcname=f'niri-motion-background-0.1.0/{name}', filter=include)
with archive.open('rb') as stream:
    digest = hashlib.file_digest(stream, 'sha256').hexdigest()
(output / 'PKGBUILD').write_text(
    (root / 'packaging/arch/PKGBUILD').read_text().replace('SOURCE_HASH', digest))
print(f'Source snapshot: {archive}\nSHA256: {digest}')
