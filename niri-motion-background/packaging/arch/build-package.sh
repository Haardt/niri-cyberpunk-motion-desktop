#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
python3 "$project_dir/packaging/arch/snapshot.py"
cd "$project_dir/dist/arch"
extra=()
# Accept a user-owned rustup toolchain as well as Arch's rust package.
if ! pacman -T rust >/dev/null 2>&1 && command -v cargo >/dev/null && command -v rustc >/dev/null; then
    command -v cc >/dev/null
    pkg-config --exists egl wayland-client
    echo 'Using the existing Rust toolchain (makepkg --nodeps); native build tools must be installed.'
    extra=(--nodeps)
fi
exec makepkg --force "${extra[@]}" "$@"
