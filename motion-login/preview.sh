#!/usr/bin/env bash
set -eu
project_dir="$(cd -- "$(dirname -- "$0")" && pwd)"
login_config="${1:-$project_dir/login.toml}"
exec niri --config "$project_dir/packaging/preview.kdl" -- "$project_dir/target/release/motion-login" --preview --config "$login_config" --background "$project_dir/background.toml"
