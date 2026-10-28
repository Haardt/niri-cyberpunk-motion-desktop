#!/bin/sh
# Own only this managed override; keep all other Noctalia settings untouched.
set -eu
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/noctalia"
override="$config_dir/niri-motion-background.toml"
marker='# Managed by niri-motion-background'
case "${1:-}" in
    start)
        if [ -e "$override" ] && ! grep -Fxq "$marker" "$override"; then
            printf 'Refusing to overwrite unmanaged %s\n' "$override" >&2
            exit 1
        fi
        mkdir -p "$config_dir"
        printf '%s\n[wallpaper]\nenabled = false\n' "$marker" > "$override"
        ;;
    stop)
        if [ -f "$override" ] && grep -Fxq "$marker" "$override"; then
            rm -- "$override"
        fi
        ;;
    *) exit 2 ;;
esac
if command -v noctalia >/dev/null 2>&1; then
    noctalia msg config-reload >/dev/null 2>&1 || true
fi
