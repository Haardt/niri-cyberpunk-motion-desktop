# niri-motion-background

> **All AI — a personal experiment.** Built and configured with coding agents as a personal test. Experimental code: use at your own risk. Upstream code/assets retain their original attribution and licenses.
>
> **The year of the Linux desktop: coding agents let you configure everything. BUT: use at your own risk — agent in YOLO mode.**

## Building the complete project set

See the [shared checkout, dependency, build-order and Arch installation guide](../README.md). Keep `motion-render` next to this directory; Cargo builds it automatically. Motion Login consumes this project's shader/license assets at packaging time but does not require the wallpaper daemon to run. Motion Lock can discover its shader from the wallpaper configuration at runtime, or use an explicit shader path.

Native Wayland animated wallpapers for niri: GPU GLSL shaders, still images and silent looping video, with battery-aware frame pacing, playlists, optional stabilization and day/night support.

The application uses the sibling [motion-render](../motion-render/README.md) Rust library. The [motion-login](../motion-login/README.md) greeter uses the same renderer and shader collection. niri and Noctalia remain separate applications; this project supplies a background surface and its own control CLI.

## Contents

- [Requirements and build](#requirements-and-build)
- [Installation and autostart](#installation-and-autostart)
- [Command-line reference](#command-line-reference)
- [Configuration](#configuration)
- [Playlist and day/night behavior](#playlist-and-daynight-behavior)
- [Shader catalog](#shader-catalog)
- [Custom shaders, images and videos](#custom-shaders-images-and-videos)
- [Performance and stabilization](#performance-and-stabilization)
- [Environment and troubleshooting](#environment-and-troubleshooting)
- [Tests and licenses](#tests-and-licenses)

## Requirements and build

Keep this directory next to `../motion-render`; the dependency is a local Cargo path:

```text
linux/
├── motion-render/
├── niri-motion-background/
└── motion-login/             # optional desktop-independent greeter application
```

Use a current stable Rust toolchain, a C linker, `pkg-config`, Wayland, and EGL/OpenGL 3.3-capable drivers. The compositor needs layer-shell and `wp_viewporter`; idle notification and fractional scaling are optional. The fullscreen heuristic additionally uses niri IPC.

On Arch/CachyOS:

```sh
sudo pacman -S --needed base-devel pkgconf wayland libglvnd
# Optional, for video backgrounds:
sudo pacman -S --needed mpv
```

Install Rust/Cargo separately if needed. The current renderer uses Wayland Client 0.31, SCTK 0.21 and glow 0.18. Locked versions are in `Cargo.lock`.

From this directory:

```sh
cargo build --release --locked
./target/release/niri-motion-background --config ./config.toml check
./target/release/niri-motion-background --config ./config.toml run
```

`check` validates TOML and enabled source paths. It does not compile shaders, open a display or verify hardware video decoding; those happen when a renderer is created.

## Arch package installation

For the full set, use the [shared build and installation guide](../README.md). This project's package helper snapshots the sibling `motion-render` directory and runs makepkg with Cargo tests/config checks:

```sh
sudo pacman -S --needed base-devel rust pkgconf python wayland libglvnd mesa
./packaging/arch/build-package.sh
sudo pacman -U ./dist/arch/niri-motion-background-0.1.0-1-x86_64.pkg.tar.zst
```

The package installs `/usr/bin/niri-motion-background`, a user service under `/usr/lib/systemd/user`, the Noctalia helper under `/usr/lib/niri-motion-background`, default config/shaders under `/usr/share/niri-motion-background`, and licenses/documentation under `/usr/share`. It does not create user configuration or enable autostart. No separate renderer package is needed. Optional video support uses `mpv`.

Initialize a new user's configuration once, preserving any existing files:

```sh
mkdir -p ~/.config/niri-motion-background/shaders
cp -n /usr/share/niri-motion-background/config.toml ~/.config/niri-motion-background/config.toml
cp -n /usr/share/niri-motion-background/shaders/*.frag ~/.config/niri-motion-background/shaders/
niri-motion-background check
systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP NIRI_SOCKET
systemctl --user daemon-reload
systemctl --user enable --now niri-motion-background.service
```

These examples use the default `~/.config` location; adjust it if you set `XDG_CONFIG_HOME`. Shader paths in the supplied config resolve relative to that config. Updates preserve your user copies: review/copy desired new assets yourself.

When migrating from `./install.sh`, `~/.local/bin/niri-motion-background` may shadow `/usr/bin/niri-motion-background`, and `~/.config/systemd/user/niri-motion-background.service` overrides the packaged unit. Check `command -v niri-motion-background` and `systemctl --user cat niri-motion-background.service`; back up and move those old local files aside before selecting the packaged binary/service. Do not run both startup methods. Existing profile/shader files remain usable.

## Installation and autostart

```sh
./install.sh
```

The installer builds the release binary and installs:

| Location | Content |
| --- | --- |
| `~/.local/bin/niri-motion-background` | CLI and background daemon |
| `${XDG_CONFIG_HOME:-~/.config}/niri-motion-background/config.toml` | Initial playlist, only when absent |
| `${XDG_CONFIG_HOME:-~/.config}/niri-motion-background/shaders/` | Missing `.frag` files only |
| `${XDG_CONFIG_HOME:-~/.config}/systemd/user/niri-motion-background.service` | User service |
| `~/.local/libexec/niri-motion-background/noctalia-wallpaper.sh` | Noctalia wallpaper override helper |
| `${XDG_DATA_HOME:-~/.local/share}/niri-motion-background/licenses/` | Attribution/license documents |

Existing configuration and shader files are preserved. Upgrading the binary does not overwrite your edited shader files or automatically replace old profiles. Review/copy desired asset/config changes yourself and use `reload`. The installer does not enable autostart.

Choose **one** startup method:

### systemd user service

Run these from a working Wayland/niri session:

```sh
systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP NIRI_SOCKET
systemctl --user daemon-reload
systemctl --user enable --now niri-motion-background.service
```

The service belongs to `graphical-session.target`, restarts on failure, and runs at `Nice=10`. Inspect it with:

```sh
systemctl --user status niri-motion-background.service
journalctl --user -u niri-motion-background.service -f
```

The service's pre/post hooks temporarily create/remove only the managed Noctalia override `~/.config/noctalia/niri-motion-background.toml`. This disables Noctalia's wallpaper renderer while the service runs and reloads its config. Other Noctalia settings/startup entries remain in place. The helper refuses to overwrite an unrelated file at that path.

### niri startup entry

Instead of the service, merge the startup entry from [packaging/niri.kdl](packaging/niri.kdl):

```kdl
spawn-at-startup "niri-motion-background" "run"
```

This direct launch does not run the systemd Noctalia hooks. If Noctalia also draws a wallpaper, configure that renderer separately or use the supplied helper. Do not start both daemon instances.

For either method, merge this layer rule into your niri configuration:

```kdl
layer-rule {
    match namespace="^niri-motion-background$"
    place-within-backdrop true
}
```

Keep the existing `layout.background-color` transparent when using backdrop composition. `packaging/niri.kdl` is a merge example, not a replacement for your full niri configuration.

Suggested bindings, inside your existing `binds` block:

```kdl
Mod+Alt+N       { spawn "niri-motion-background" "next"; }
Mod+Alt+Shift+N { spawn "niri-motion-background" "prev"; }
Mod+Alt+P       { spawn "niri-motion-background" "toggle"; }
Mod+Alt+A       { spawn "niri-motion-background" "auto-switch" "toggle"; }
```

Use the absolute `~/.local/bin` executable path if it is absent from the session's PATH. Stopping/disabling the service restores its managed Noctalia override:

```sh
systemctl --user disable --now niri-motion-background.service
```

## Command-line reference

```text
niri-motion-background [-c FILE | --config FILE] COMMAND
```

| Command / option | Behavior |
| --- | --- |
| `run` | Start the foreground Wayland background daemon |
| `check` | Validate the selected configuration and enabled local source files |
| `status` | JSON status: current entry, FPS/scale, power, pause, period, rotation and per-output state |
| `list` | JSON playlist, including disabled entries, entry limits, notes and eligibility |
| `set NAME` | Select any enabled entry by exact name |
| `next` / `prev` | Cycle through the current eligible playlist |
| `pause` / `resume` | Set/clear manual pause |
| `toggle` | Toggle manual pause |
| `reload` | Reload the daemon's original config file and stage renderer changes |
| `quit` | Stop the daemon |
| `mode auto\|day\|night` | Set a runtime period override; `auto` removes it |
| `auto-switch on\|off\|toggle` | Change runtime automatic rotation |
| `-c FILE`, `--config FILE` | Config path for `run`/`check`; default is the XDG config path below |
| `-h`, `--help` | CLI help, also available for subcommands |
| `-V`, `--version` | Application version |

The default file is `$XDG_CONFIG_HOME/niri-motion-background/config.toml`, or `~/.config/niri-motion-background/config.toml` when the variable is unset.

Control commands address the running daemon through IPC. `--config` does not create a separate instance or choose a different control socket, and `reload` always reads the file the daemon started with. Use `MOTION_RENDER_SOCKET` to isolate instances.

Runtime choices do not rewrite TOML. To preserve a choice across restarts, edit `active`, `[rotation]` or `[day_night]` and reload. `resume` removes manual pause only; idle/fullscreen/battery/hidden conditions can still pause rendering. A failed synchronous renderer/config reload leaves the previous working renderer selected. Later asynchronous video failures are exposed through status.

## Configuration

See [config.toml](config.toml) for the complete shipped playlist and the [shared configuration reference](../motion-render/README.md#configuration-reference) for every default, field and validation rule.

This example includes all global, rotation/day-night and entry options:

```toml
fps = 24
battery_fps = 12
render_scale = 0.4
battery_render_scale = 0.32
pause_on_idle = true
idle_seconds = 120
pause_on_battery = false
pause_when_hidden = true
hidden_timeout_ms = 750
pause_on_fullscreen = true
hwdec = "auto-safe"
require_hardware_video = false
active = "cyberpunk-city-rain"

[rotation]
enabled = true
interval_seconds = 600
defer_while_paused = true

[day_night]
enabled = true
source = "noctalia"
day_start = "07:00"
night_start = "20:00"
day_brightness = 1.0
night_brightness = 0.65

[[wallpapers]]
name = "cyberpunk-city-rain"
type = "shader"
source = "shaders/cyberpunk-city-rain.frag"
enabled = true
period = "any"
speed = 0.35
brightness = 1.0
fps = 20
render_scale = 0.32
stabilize = true
note = "Aerial neon city with rain"

[[wallpapers]]
name = "my-image"
type = "image"
source = "images/wallpaper.png"
enabled = false

[[wallpapers]]
name = "my-video"
type = "video"
source = "videos/background.mp4"
enabled = false
```

Key rules:

- Global/entry FPS accepts `1..240`; scales accept `0.1..1.0`. Battery and entry values can reduce global work, never increase it.
- Source paths are relative to the TOML file. Enabled sources must exist; disabled templates may reference absent files. Use local files, not URLs.
- Entry names must be unique/nonempty, and `active` must select an enabled entry.
- `speed` accepts `0..10`, `brightness` and day/night multipliers `0.05..2.0`, all finite. `speed = 0` freezes shader time.
- `idle_seconds` and rotation interval must be positive; `hidden_timeout_ms` must be at least `100`.
- `period` is `any`, `day` or `night`. Automatic day/night requires a usable entry for each period, or an `any` entry.
- Unknown renderer keys are errors. Put top-level keys before `[section]`/`[[wallpapers]]` blocks.

## Playlist and day/night behavior

Rotation uses enabled entries for the current period in configuration order. It wraps, skips synchronous load errors and avoids reinitializing a single-entry playlist. An explicit `set` may select an entry outside its period; subsequent automatic selection uses the eligible pool again.

With `defer_while_paused = true`, an expired rotation waits while all outputs are inactive. Resuming advances once, not once for every missed interval. `next`, `prev`, `set`, reload and rotation toggles reset the rotation deadline. `status.next_switch_seconds = 0` during pause can therefore mean a deferred switch.

`source = "noctalia"` follows Noctalia's resolved light/dark mode. Queries happen on a worker, at most every 30 seconds, with a two-second timeout. The local clock schedule is the fallback. `source = "schedule"` uses local time without querying Noctalia. Schedules can cross midnight and do not calculate astronomical sunrise/sunset.

Manual `mode` changes affect this background only. Entry and period brightness multiply for shaders/images. Video colors and playback speed remain unchanged; separate day/night video entries can be used instead.

## Shader catalog

The shipped wallpaper playlist has **51 enabled shaders** and one disabled local-video template. The caps below are entry caps, further limited by global/battery settings; `global` means no entry-specific cap. The login's independent playlist also includes these 51 shaders.

| Name | Scene | Speed | FPS cap | Scale cap | Stabilize |
| --- | --- | ---: | ---: | ---: | --- |
| `topo` | Animated topographic contours | 0.1 | global | global | no |
| `plasma` | Flowing plasma colors | 0.08 | global | global | no |
| `mandelbrot-prism-flight` | Mandelbrot coast spiral, chrome relief and rainbow neon | 0.6 | 24 | 0.32 | yes |
| `silk-continuum` | Fullscreen opalescent folds, translucent veils and immersive depth/parallax | 0.6 | 20 | 0.4 | no |
| `oneiric-silk` | Pearlescent trefoil dream sculpture, reflected folds, flowing light threads | 0.6 | 24 | 0.5 | no |
| `retro-boing` | 1980s checker ball demo, copper bars and CRT stage | 0.5 | 24 | 0.5 | no |
| `disco-slowmo` | Slow mirror ball, colorful beams, star glints and reflective dance floor | 0.35 | 24 | 0.5 | yes |
| `algebra-of-love` | Five-chapter retro scientific terminal: AST, geometry, chaos, Life/network, heart equation | 0.65 | 24 | 0.5 | no |
| `threshold-of-self` | Fictional consciousness metaphor: braided light pressing through a segmented glass shell | 0.65 | 24 | 0.5 | yes |
| `almost-touching` | Curved cyan and coral light membranes, brief connecting filament and stage reflections | 0.7 | 24 | 0.5 | yes |
| `world-execute` | Breathing glass landscape with flowing height contours; always visible, no coordinate cross | 0.6 | 24 | 0.32 | yes |
| `tangent-garden` | Gold and coral botanical curves with fine petal contours and traveling tangents | 0.65 | 24 | 0.55 | no |
| `memory-of-you` | Dark reflective liquid with a traveling light ribbon and fading time-lagged wakes | 0.65 | 24 | 0.5 | no |
| `strange-simulation` | Translucent glass galleries with divergent landscapes that periodically align | 0.65 | 24 | 0.5 | no |
| `bioluminescent-ocean` | Bioluminescent waves and broad drifting underwater lights | 0.65 | 24 | 0.4 | no |
| `rainy-window` | Refracting rain beads, wet trails and warm defocused city lights | 0.65 | 24 | 0.4 | no |
| `emerald-glass-forest` | Faceted emerald glass trees, mist and soft shafts of sunlight | 0.65 | 20 | 0.32 | yes |
| `ferrofluid` | Black magnetic liquid with moving spikes and studio reflections | 0.65 | 20 | 0.32 | yes |
| `autumn-koi-pond` | Abstract koi ribbons, translucent flowing tails, quiet maple leaves and rain rings | 0.65 | 24 | 0.4 | no |
| `infinite-library` | Endless bookshelves, warm lamps, arches and drifting dust | 0.65 | 20 | 0.36 | yes |
| `crystal-cavern` | Pulsing violet and cyan crystals reflected in a dark cave pool | 0.65 | 20 | 0.32 | yes |
| `event-horizon-feast` | Lensed black hole with dense accretion filaments and feeding streams | 0.5 | 20 | 0.4 | yes |
| `green-chrome-cubes` | Glossy green chrome cube sea with stacks and reflections | 0.5 | 20 | 0.4 | yes |
| `wildflower-meadow` | Windblown meadow with colorful wildflowers | 0.65 | 24 | 0.5 | no |
| `aurora-curtains` | Layered turquoise/violet aurora curtains | 0.35 | 20 | 0.4 | no |
| `aurora` | Aurora-like light ribbons | 0.1 | global | global | no |
| `gradient` | Animated color gradient | 0.12 | global | global | no |
| `ai1` | Original rosewallpapers AI1 shader | 0.1 | global | global | no |
| `eclipse-corona` | Solar eclipse and corona | 0.32 | global | global | no |
| `smoke-ring` | Glowing smoke/fire ring | 0.12 | global | global | no |
| `god-rays` | Radial light rays | 0.16 | global | global | no |
| `neuro-noise` | Blue cellular/neural noise | 0.07 | global | global | no |
| `black-hole` | Procedural black-hole illustration | 0.07 | global | global | no |
| `nebula` | Procedural nebula | 0.12 | global | global | no |
| `arrakis` | Desert landscape and sun | 0.24 | global | global | no |
| `ocean-sunrise` | Choppy ocean, reflections, foam and a looping solar cycle | 0.5 | global | global | no |
| `corroded-beasts` | Organic corroded tunnel | 0.055 | 12 | 0.28 | yes |
| `fractal-cartoon` | Fractal scene with a procedural cat sprite | 0.045 | 15 | 0.28 | yes |
| `cube-lines` | Transparent geometric layers in a rotating cube | 0.075 | 20 | 0.32 | yes |
| `synthwave-sunset` | Retro triangular terrain and sunset | 0.06 | 20 | 0.3 | yes |
| `mandelbulb` | Rotating Mandelbulb fractal | 0.05 | 12 | 0.26 | yes |
| `seascape-tdm` | TDM Seascape ocean port | 0.15 | 20 | 0.22 | no |
| `cloud-orbit` | Orbiting volumetric cloud | 0.04 | 12 | 0.16 | no |
| `cloud-sunlit` | Sunlit volumetric cloud | 0.04 | 12 | 0.16 | no |
| `neon-menger` | Reflective neon fractal | 0.045 | 12 | 0.24 | yes |
| `blue-rectangles` | Blue noise field and floating rectangles | 0.06 | 20 | 0.25 | no |
| `sky-path` | Steady flight through geometric pathways | 0.18 | 15 | 0.32 | yes |
| `inercia-royale-2022` | Reflective folded geometry | 0.045 | 12 | 0.24 | yes |
| `portal-turret` | Procedural sentry turret fan-art | 0.35 | 20 | 0.32 | yes |
| `cyberpunk-city-flight` | Aerial neon city, wet roofs/glass, no falling rain | 0.35 | 20 | 0.32 | yes |
| `cyberpunk-city-rain` | The same aerial city with three layers of rain | 0.35 | 20 | 0.32 | yes |

The [Mandelbrot flight gallery](assets/mandelbrot-prism-flight-gallery.png) shows the overview, coast approach and deep detail. It uses native fp64 when available, otherwise double-single arithmetic and a shallower zoom. The native path reaches about 4,200x magnification during a continuous eight-minute logarithmic zoom cycle at the shipped speed. Perspective tilt, bank and a local orbit keep the route moving; distance filtering calms the finest neon contours. GPU checks compare membership to a CPU f64 reference and profile the demanding flight phases. A [before/after detail](assets/mandelbrot-prism-flight-smoothing.png) shows the calmer version: a fixed nine-tap Gaussian footprint, broader color bands and temporal stabilization calm fine structures. This preset keeps the same pixel grid on AC and battery to avoid resampling jumps when the power source changes.

The [Silk Continuum preview](assets/silk-continuum-preview.png) shows an edge-to-edge environment: the camera moves inside endless opalescent folds and glass veils. Portrait uses the same full-frame perspective; there is no central sculpture, frame, floor or vignette. GPU checks include full-field coverage and rendering costs at 1152x768.

The [Oneiric Silk preview](assets/oneiric-silk-preview.png) shows an abstract pearlescent ribbon sculpture. Its continuous trefoil geometry has reflected folds, a slowly changing camera and light threads; four spatial samples keep the contour stable.

The [Retro Boing preview](assets/retro-boing-preview.png) shows an original demo-scene homage with antialiased checkers, a slow bounce, clearly traveling/spreading copper bars and a subtle CRT bezel. It needs no textures or original demo assets.

A [Disco Slowmo preview](assets/disco-slowmo-preview.png) shows the mirror ball. Its rotation takes about eight minutes at the shipped speed; all glints move continuously, with no strobe animation.

[Algebra of Love's chapter gallery](assets/algebra-of-love-gallery.png) shows the terminal sequence. Its 90-second shader-time loop takes about 138 seconds at the shipped speed. Lorenz and double-pendulum traces use RK4 numerical integration, Conway frames use B3/S23, and the neural network is an illustration. The heart plot uses the exact zero set of the displayed equation. To regenerate its original 5x7 font, numerical data and standalone shader after editing `shaders/lib/algebra-of-love.glsl`, run `python3 tools/build-algebra-of-love.py`.

The [Threshold of Self triptych](assets/threshold-of-self-triptych.png) shows containment, pressure and breakthrough.

The five mathematical/emotional concepts have their own [preview gallery](assets/concept-scenes-gallery.png).

A [preview gallery](assets/new-scenes-gallery.png) shows the seven nature, weather and material scenes.

The city variants share [shaders/lib/cyberpunk-city.glsl](shaders/lib/cyberpunk-city.glsl). After editing it, run `python3 tools/build-city.py` to regenerate both self-contained `.frag` files; no Python or shader includes are needed at runtime.

Ocean Sunrise's artistic day/sunset/night cycle is independent of real-time theme selection. The Fractal Cartoon cat is generated procedurally; it needs no external sprite texture. Shader authorship and asset-specific restrictions are listed in [THIRD_PARTY.md](THIRD_PARTY.md).

## Custom shaders, images and videos

Add a file, add a matching entry, then validate/reload:

```sh
niri-motion-background check
niri-motion-background reload
niri-motion-background set my-background
```

For GLSL, use the [renderer shader interface](../motion-render/README.md#shader-interface): GLSL 330 core, `void main(`, output `fragColor`, and optional `u_time`, `u_resolution`, `u_night`. Shadertoy code requires a port; arbitrary buffers, audio and `iChannel` textures are not imported automatically.

PNG, JPEG and WebP images are aspect-filled and do not redraw periodically. Videos loop silently through libmpv; audio/subtitles/OSD/default input bindings and mpv configuration files are disabled.

Supported decoder selectors are `auto-safe`, `auto`, `no`, `vaapi`, `nvdec`, and `vulkan`. Availability depends on the codec, mpv and driver. `vulkan` selects a decoder mode, not a different rendering backend. `require_hardware_video = true` rejects software decoding. Inspect `status.outputs[].decoder` and `video_failed`; hardware decode failure can be asynchronous after selection.

## Performance and stabilization

FPS is a presentation ceiling, not a display refresh-rate setting or a decoder-FPS guarantee. Reducing `render_scale` reduces pixel workload quadratically; video decoding may still happen at the source dimensions/rate. The shipped global profile is 24 FPS/0.4 scale on AC and 12 FPS/0.32 on battery, before entry caps.

`stabilize = true` applies a GPU-only spatial filter and neighborhood-clamped temporal history. It reduces isolated bright pixels and shimmering thin geometry at the cost of softness, memory and additional GPU passes. History resets on scene, size, brightness and period changes. It does not apply to images or video, and does not perform CPU readbacks.

Rendering follows compositor callbacks and monotonic deadlines; missed slots are skipped. Paused animation time freezes. Battery detection is sampled every 30 seconds. Hidden/fullscreen detection is heuristic: hidden state follows delayed frame callbacks, and fullscreen-sized tiles are inferred from niri's active workspace.

## Environment and troubleshooting

| Variable | Meaning |
| --- | --- |
| `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR` | Wayland connection/runtime directory |
| `XDG_CONFIG_HOME` | Default configuration directory |
| `XDG_DATA_HOME` | Installer's license/document directory base |
| `NIRI_SOCKET` | niri event-stream/fullscreen integration |
| `MOTION_RENDER_SOCKET` | Independent IPC socket filename; default `niri-motion-background.sock` |
| `RUST_LOG` | Logging filter, for example `debug` |

The same-user control socket is mode `0600` and protected by an instance lock. For isolated testing, export a separate `MOTION_RENDER_SOCKET` for both daemon and clients.

| Symptom | Check |
| --- | --- |
| No daemon / connection refused | `systemctl --user status`, runtime directory and service environment |
| Duplicate instance rejected | Keep one startup method; inspect service and niri startup entries |
| Background hidden behind a solid color | niri layer rule, transparent layout background and competing wallpaper renderers |
| Config reload fails | `check`, relative source paths and strict key spelling; old working state is retained |
| Shader compilation fails | GLSL 330 interface and `fragColor`/entry-point names; inspect logs |
| Video fails or reports software | Install mpv/driver, use a supported local codec, inspect decoder and strict-hardware setting |
| Wallpaper remains paused after `resume` | Check idle, fullscreen, battery and hidden conditions in `status` |
| Asset edits do not appear after reinstall | Installer preserves existing shader files; update the installed copy deliberately |
| New rotation/theme settings disappear after restart | Runtime control commands are not persistent; edit TOML |

Functional validation used Intel Arc B390, Mesa and VAAPI. Physical multi-monitor hotplug, suspend/resume and other GPU vendors need hardware-specific validation. No whole-system battery-life or wattage guarantee is made.

## Tests and licenses

```sh
cargo test --locked
cargo clippy --all-targets -- -D warnings
# Policy tests live in the shared library:
cargo test --manifest-path ../motion-render/Cargo.toml --locked
# Real offscreen EGL/OpenGL stabilization regression:
cargo test --test stabilization -- --ignored --nocapture
# Optional capture output:
NMB_CAPTURE_DIR=/tmp cargo test --test stabilization -- --ignored --nocapture
```

The concept scenes, abstract koi and Threshold of Self have an opt-in real GPU check for opaque output, contrast, animation continuity, portrait rendering and stabilization. To export large previews at three animation stages:

```sh
MOTION_PREVIEW_DIR=/tmp/concept-previews cargo test --test concept_scenes -- --ignored --nocapture
```

The live smoke test creates temporary configurations and draws real backgrounds. Run it in a dedicated/nested Wayland session, or after stopping the ordinary daemon; it refuses an already occupied control socket:

```sh
python3 -u tests/smoke.py
python3 -u tests/smoke.py --binary ./target/release/niri-motion-background --video /absolute/path/test.mp4
```

It covers all enabled shaders, duplicate-instance rejection, frame ceilings, idle CPU behavior during manual pause, static images, invalid-load rollback, reload, rotation and day/night filtering. Video checks require libmpv and the supplied local test file. An isolated socket can be selected through `MOTION_RENDER_SOCKET` without interrupting the desktop daemon; still use a separate compositor to avoid replacing the visible desktop background.

Application/library code is [MIT-licensed](LICENSE). Shader licenses differ: original/MIT assets, Apache-2.0 ports, and user-submitted assets retaining notices such as CC BY-NC-SA 3.0. The root MIT license does not relicense them. See [THIRD_PARTY.md](THIRD_PARTY.md), [third_party/](third_party/) and the notices in individual shaders.
