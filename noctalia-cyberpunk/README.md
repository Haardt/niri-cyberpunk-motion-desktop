# Noctalia Cyberpunk

> **All AI — a personal experiment.** These local customizations were built with coding agents. Use at your own risk — agent in YOLO mode. Upstream code and assets retain their attribution and licenses.

A fork of Noctalia with angular cyberpunk panels, a yellow/cyan/red palette, scoped glitch animations when panels and the launcher open, ambient sweeps on individual bar widgets, and animated tooltips. It uses Noctalia's existing GLES renderer and keeps hit regions stationary.

Part of the [Motion Desktop monorepo](../README.md). This shell builds independently of the four Rust projects. The [upstream README](README.upstream.md), [build guide](BUILDING.md), [packaging notes](PACKAGING.md), [credits](CREDITS.md) and [MIT license](LICENSE) are preserved.

## Source and provenance

- Upstream: https://github.com/noctalia-dev/noctalia
- Base version: 5.3.0
- Base commit: `a33122c4208a385e19c9a252e65e2751b2c44fbb`
- Imported from the local `cyberpunk/prototype` checkout using `cp -a`; the original checkout remains untouched.
- `NOCTALIA_CYBERPUNK=1` enables the renderer changes. `NOCTALIA_CYBERPUNK_LIVE=1` hides the development indicator.

This is a source snapshot in the monorepo, not a Git submodule. It does not automatically follow upstream releases. The installed system Noctalia may be newer than this fork; the package deliberately coexists with it.

## Arch/CachyOS dependencies

A C++23 compiler (GCC 13+), Meson 1.1+, Ninja, Python and these native dependencies are needed. Cargo is not involved.

```sh
sudo pacman -S --needed base-devel meson ninja pkgconf python \
  wayland wayland-protocols libglvnd freetype2 fontconfig cairo pango harfbuzz \
  libxkbcommon glib2 libsecret libsodium sdbus-cpp libpipewire wireplumber \
  polkit pam curl libwebp libjxl libsndfile librsvg libqalculate libxml2 \
  md4c tomlplusplus libical nlohmann-json stb jemalloc
```

The source contains the vendored Luau, Wuffs, fzy and Material Color Utilities dependencies. niri and D-Bus are needed for the nested preview; PipeWire/WirePlumber provide normal audio integration. Optional desktop services include upower and power-profiles-daemon. The shell's own fingerprint integration uses fprintd; Motion Login and Motion Lock keep their separate PAM policies.

## Build and preview without replacing the running shell

```sh
cd ~/workspaces/linux/motion-desktop/noctalia-cyberpunk
meson setup build-cyberpunk --buildtype=debugoptimized -Dtests=enabled
meson compile -C build-cyberpunk -j 4 noctalia cyberpunk_effect_test
meson test -C build-cyberpunk cyberpunk_effect --print-errorlogs
./tools/cyberpunk-preview
```

The preview creates a nested niri window and private D-Bus session. Never add `--session` to the nested compositor. It has its own config/state/cache and disables authentication, notifications, clipboard, plugins and template application. It does not require this workstation's plugin cache. Alt+Space opens the launcher, Alt+C opens the clock panel, and Alt+Shift+Q exits. `./tools/cyberpunk-preview --baseline` shows the theme without the renderer effect.

The current live profile is in `cyberpunk/profile/`; the isolated preview uses the deliberately limited `cyberpunk/config/`. The [effect notes](docs/cyberpunk-prototype.md) describe timing and rendering details.

## Build an Arch package, then install with pacman

```sh
./packaging/arch/build-package.sh
sudo pacman -U ./dist/arch/noctalia-cyberpunk-5.3.0-1-x86_64.pkg.tar.zst
```

Set `BUILD_JOBS=4` (the default) to control parallel compilation. The helper creates a source archive and SHA-256 checksum, then invokes makepkg. Its check step runs the scoped-effect regression test and validates the portable preview configuration. Use the filename matching your architecture; the recipe supports x86_64 and aarch64.

The package installs these distinct paths:

| Path | Purpose |
| --- | --- |
| `/usr/bin/noctalia-cyberpunk` | Wrapper enabling the cyberpunk renderer and selecting matching assets |
| `/usr/lib/noctalia-cyberpunk/noctalia` | Fork executable |
| `/usr/share/noctalia-cyberpunk/assets/` | Matching upstream assets and translations |
| `/usr/share/noctalia-cyberpunk/profile/` | Example live bar, panel styling and palette |
| `/usr/share/doc/noctalia-cyberpunk/` | Fork and upstream documentation |

`/usr/bin/noctalia`, the upstream desktop entry, user configuration and autostart are not replaced. The package does not start or stop any shell. Do not run two full shells in the same desktop session.

## Apply the example profile and choose the fork

Back up your Noctalia configuration first. Copy the profile fragments without overwriting existing files:

```sh
mkdir -p ~/.config/noctalia/palettes
cp -n /usr/share/noctalia-cyberpunk/profile/*.toml ~/.config/noctalia/
cp -n /usr/share/noctalia-cyberpunk/profile/palettes/*.json ~/.config/noctalia/palettes/
```

Review existing fragments and `~/.local/state/noctalia/settings.toml`: saved overrides can take precedence over the example theme. Keep your existing authentication/idle configuration. The Motion Lock menu/idle/logind integration is documented separately in the [locker README](../motion-lock/README.md).

In niri, replace the existing Noctalia startup command with this entry rather than adding a second shell:

```kdl
spawn-at-startup "/usr/bin/noctalia-cyberpunk"
```

Validate with `niri validate`, then log out/in normally to select the new shell. Once it is running, choose the palette:

```sh
noctalia-cyberpunk msg color-scheme-set custom Cyberpunk-Prototype
```

The wrapper's `msg` commands address the normal Noctalia instance. Existing shortcuts can continue using `noctalia msg ...`; on systems with mismatched CLI/IPC versions, use `noctalia-cyberpunk msg ...` explicitly.

The bar profile refers to the optional `noctalia/screen_recorder` plugin. Install a compatible plugin version through Noctalia's plugin manager if desired, or remove the `screen-recorder` widget from the bar. The plugin and gpu-screen-recorder are not bundled; the core shell, launcher, CPU widget and glitch effects do not require them.

## Return to the upstream shell

Restore your original niri startup entry and backed-up profile/settings, then log out/in normally. Remove this fork package with `sudo pacman -R noctalia-cyberpunk` when no longer needed. It does not replace the system Noctalia package. The original workstation's `~/.local/bin/noctalia` wrapper and `~/.local/libexec/noctalia-cyberpunk` installation are separate from this package; review any existing wrapper when migrating.
