#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"
cargo build --release --locked
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/niri-motion-background"
unit_dir="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
libexec_dir="$HOME/.local/libexec/niri-motion-background"
license_dir="${XDG_DATA_HOME:-$HOME/.local/share}/niri-motion-background/licenses"
mkdir -p "$HOME/.local/bin" "$config_dir/shaders" "$unit_dir" "$libexec_dir" "$license_dir/paper" "$license_dir/subframe" "$license_dir/user-submitted"
install -m755 target/release/niri-motion-background "$HOME/.local/bin/niri-motion-background.new"
mv -f -- "$HOME/.local/bin/niri-motion-background.new" "$HOME/.local/bin/niri-motion-background"
install -m755 packaging/noctalia-wallpaper.sh "$libexec_dir/noctalia-wallpaper.sh"
install -m644 LICENSE THIRD_PARTY.md "$license_dir/"
install -m644 third_party/paper/LICENSE third_party/paper/NOTICE "$license_dir/paper/"
install -m644 third_party/subframe/LICENSE "$license_dir/subframe/"
install -m644 third_party/user-submitted/ATTRIBUTION.md "$license_dir/user-submitted/"
if [[ ! -e "$config_dir/config.toml" ]]; then
    install -m644 config.toml "$config_dir/config.toml"
fi
for shader in shaders/*.frag; do
    destination="$config_dir/shaders/${shader##*/}"
    if [[ ! -e "$destination" ]]; then
        install -m644 "$shader" "$destination"
    fi
done
install -m644 packaging/niri-motion-background.service "$unit_dir/niri-motion-background.service"
printf 'Installed. Existing config and shaders preserved. Autostart has not been enabled.\n'
printf 'See README.md for niri and Noctalia integration.\n'
