#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "$0")/../.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
python3 "$project_dir/packaging/arch/snapshot.py"
cd "$project_dir/dist/arch"
# Runtime greetd is intentionally not installed during a local package build.
# Rust/pkgconf and all actual build tools must already be available.
exec makepkg --nodeps --force "$@"
