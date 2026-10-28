#!/usr/bin/env python3
"""Snapshot the fork sources without Git metadata, caches or build products."""
from pathlib import Path
import tarfile
import hashlib
root = Path(__file__).resolve().parents[2]
version = (root / 'VERSION').read_text().strip()
output = root / 'dist/arch'
output.mkdir(parents=True, exist_ok=True)
archive = output / f'noctalia-cyberpunk-{version}.tar.gz'
def include(info):
    parts = Path(info.name).parts[1:]
    return None if any(p in {'.git', 'dist', '__pycache__', '.cache'} or (info.isdir() and p.startswith('build')) for p in parts) else info
with tarfile.open(archive, 'w:gz') as tar:
    tar.add(root, arcname=f'noctalia-cyberpunk-{version}', filter=include)
with archive.open('rb') as stream:
    digest = hashlib.file_digest(stream, 'sha256').hexdigest()
(output / 'PKGBUILD').write_text((root / 'packaging/arch/PKGBUILD').read_text().replace('SOURCE_HASH', digest))
print(f'Source archive: {archive}\nSHA256: {digest}')
