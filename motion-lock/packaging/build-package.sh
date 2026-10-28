#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.."
# Snapshot only this project. Never copy changing sibling workspaces during builds.
python3 - <<'PY'
from pathlib import Path
import tarfile
root=Path.cwd()
with tarfile.open(root/'motion-lock-src.tar.gz','w:gz') as archive:
    for rel in ['Cargo.toml','Cargo.lock','LICENSE','README.md','THIRD-PARTY.txt','config.example.toml','src','fonts','vendor','packaging','tests','.gitignore']:
        archive.add(root/rel,arcname='motion-lock/'+rel)
PY
export CARGO_TARGET_DIR="$PWD/target"
mkdir -p .package
cp PKGBUILD motion-lock-src.tar.gz .package/
cd .package
python3 - <<'PY2'
from pathlib import Path
import hashlib
p=Path('PKGBUILD');digest=hashlib.sha256(Path('motion-lock-src.tar.gz').read_bytes()).hexdigest()
p.write_text(p.read_text().replace("sha256sums=('SKIP')", "sha256sums=('"+digest+"')"))
PY2
extra=()
# This workstation has a working user-owned rustup toolchain, outside pacman's DB.
if ! pacman -T cargo >/dev/null 2>&1 && command -v cargo >/dev/null; then
    pkg-config --exists egl wayland-client xkbcommon
    command -v clang >/dev/null
    echo 'Using the existing user-owned Rust toolchain (makepkg --nodeps).'
    extra=(--nodeps)
fi
makepkg --force "${extra[@]}" "$@"
while IFS= read -r package; do cp -- "$package" ../; done < <(makepkg --packagelist)

