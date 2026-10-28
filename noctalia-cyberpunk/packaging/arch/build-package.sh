#!/usr/bin/env bash
set -euo pipefail
project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
python3 "$project_dir/packaging/arch/snapshot.py"
cd "$project_dir/dist/arch"
exec makepkg --force "$@"
