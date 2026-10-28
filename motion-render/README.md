# motion-render

> **All AI — a personal experiment.** Built and configured with coding agents as a personal test. Experimental code: use at your own risk. Upstream code/assets retain their original attribution and licenses.
>
> **The year of the Linux desktop: coding agents let you configure everything. BUT: use at your own risk — agent in YOLO mode.**

## Building the complete project set

See the [shared checkout, dependency, build-order and Arch installation guide](../README.md) for the complete monorepo. This crate has no standalone executable or Arch package: Cargo compiles it into the wallpaper and login binaries through `../motion-render`. Motion Lock uses a separate vendored snapshot; changing this directory does not update that copy.

A Rust library for native Wayland GPU backgrounds. It provides EGL/OpenGL rendering, GLSL shaders, still images, optional hardware-decoded video, and a battery-aware layer-shell runtime.

Used by the sibling applications:

- [niri-motion-background](../niri-motion-background/README.md): desktop wallpaper daemon and control CLI.
- [motion-login](../motion-login/README.md): greetd login interface with an animated background.

This is a library, not a display manager or an executable. It contains no PAM, password verification, or fingerprint handling. Rust consumers link it into their binaries; no separate system-wide shared-library package is required.

## Contents

- [Project layout and requirements](#project-layout-and-requirements)
- [Building and using the library](#building-and-using-the-library)
- [Public modules and resource ownership](#public-modules-and-resource-ownership)
- [Configuration reference](#configuration-reference)
- [Shader interface](#shader-interface)
- [Images and video](#images-and-video)
- [Scheduling, power and desktop integration](#scheduling-power-and-desktop-integration)
- [IPC and environment](#ipc-and-environment)
- [Validation and licensing](#validation-and-licensing)

## Project layout and requirements

Keep the three directories next to one another. Consumers currently use Cargo path dependencies, rather than a published crates.io release:

```text
linux/
├── motion-render/
├── niri-motion-background/
└── motion-login/
```

Build requirements are a current stable Rust toolchain, a C linker, `pkg-config`, and Wayland/EGL development libraries. Runtime rendering requires Linux, a Wayland compositor with layer-shell support, and an EGL/OpenGL 3.3-capable driver. The complete background runtime also requires `wp_viewporter`; fractional scaling and idle notification are negotiated when available.

On Arch/CachyOS, the native build dependencies can be installed with:

```sh
sudo pacman -S --needed base-devel pkgconf wayland libglvnd
```

Install Rust/Cargo separately through your preferred toolchain manager. Shader/image rendering does not require mpv. Video additionally requires a compatible `libmpv.so.2`, normally provided by Arch's `mpv` package, and a suitable GPU decoder/driver.

The current stack uses `wayland-client` 0.31, `wayland-protocols` 0.32, Smithay Client Toolkit 0.21, `glow` 0.18, TOML 1.1, and `libloading` 0.9. Exact resolved versions are recorded in each project's `Cargo.lock`.

## Building and using the library

From this directory:

```sh
cargo build --release --locked
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo doc --no-deps --open
```

Add it to a sibling crate:

```toml
[dependencies]
motion-render = { path = "../motion-render" }
anyhow = "1"
```

Load and validate a playlist:

```rust
use std::path::Path;
use motion_render::config::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::load(Path::new("../niri-motion-background/config.toml"))?;
    println!("{} configured backgrounds", config.wallpapers.len());
    Ok(())
}
```

To run the complete background client, use `motion_render::app::run(config_path)`. This is a blocking event/render loop: it owns its Wayland connection, opens IPC, and installs process signal handlers. Treat it as a once-per-process runtime. The login supervisor runs it in a separate child process; a custom compositor client can instead use the lower-level EGL/GPU APIs.

## Public modules and resource ownership

| Module | Purpose / entry points |
| --- | --- |
| `app` | Complete per-output layer-shell background runtime; `run(PathBuf)` |
| `config` | `Config::load`, `validate`, playlist types, effective FPS/scale and clock parsing |
| `egl` | `Display::new`, `Display::with_alpha`, `surface`, `current`, `gl`; surface `resize`/`swap` |
| `gpu` | `Gpu::shader`, `Gpu::image`, `draw`, explicit `destroy` |
| `video` | Dynamically loaded libmpv; `Video::new`, `update`, `pending`, `pause`, `draw`, `swapped` |
| `pacing` | Monotonic frame deadlines and pause-aware animation steps |
| `selection` | Day/night resolution, eligible entries, adjacent selection and rotation deadlines |
| `power` | Battery detection from `/sys/class/power_supply` |
| `niri` | niri event-stream watcher and active-workspace tile tracking |
| `theme` | Bounded Noctalia theme queries on a worker thread |
| `ipc` | Same-user Unix socket server and `send(command)` client |
| `wake` | Linux eventfd wrapper: `new`, `fd`, `notify`, `drain` |

Keep EGL contexts, surfaces and OpenGL calls on their owning render thread. Make the correct surface current before GPU work. Call `Gpu::destroy` while its context is current; then destroy EGL windows before their Wayland surfaces, and destroy the EGL display last. The complete `app` runtime handles this ordering.

libmpv render calls also stay on the EGL thread. Its callbacks only set an atomic flag and signal an eventfd; they do not issue OpenGL calls. `Video::new` expects a current GL context and the native Wayland display pointer, not an arbitrary EGL display handle.

## Configuration reference

The same TOML format is used by desktop `config.toml` and login `background.toml`. Keys use **snake_case**. Paths are resolved relative to the configuration file, not the working directory. Enabled sources must be existing local files. Unknown keys in the renderer schema are rejected.

### Global settings

These are Rust schema defaults. Applications may ship different values; inspect their example files.

| Key | Default | Meaning / accepted values |
| --- | --- | --- |
| `fps` | `24` | Global presentation ceiling, integer `1..240` |
| `battery_fps` | `12` | Battery ceiling, `1..240`; cannot raise the global ceiling |
| `render_scale` | `0.4` | Buffer dimensions relative to native output pixels, `0.1..1.0` |
| `battery_render_scale` | `0.32` | Battery scale, `0.1..1.0`; cannot raise global resolution |
| `pause_on_idle` | `true` | Use compositor idle notifications when available |
| `idle_seconds` | `120` | Positive idle timeout in seconds |
| `pause_on_battery` | `false` | Stop animation while discharging, rather than merely reducing limits |
| `pause_when_hidden` | `true` | Pause when compositor frame callbacks remain outstanding |
| `hidden_timeout_ms` | `750` | Hidden-callback threshold, at least `100` ms |
| `pause_on_fullscreen` | `true` | Pause an output when niri tile layout indicates a fullscreen-sized window |
| `hwdec` | `"auto-safe"` | `auto-safe`, `auto`, `no`, `vaapi`, `nvdec`, or `vulkan`; libmpv decoder choice |
| `require_hardware_video` | `false` | Reject/pause a video if libmpv reports software decoding |
| `active` | `"topo"` | Name of the initial enabled playlist entry |
| `wallpapers` | empty | One or more `[[wallpapers]]` entries are required |
| `rotation` | disabled | Nested `[rotation]` settings described below |
| `day_night` | disabled | Nested `[day_night]` settings described below |

`Config::default()` alone is not a valid playable configuration: add at least one entry matching `active`.

Effective FPS is the minimum of the global ceiling, battery ceiling when applicable, and the current entry's optional ceiling. Scale follows the same minimum rule. A scale of `0.5` means approximately one quarter of the native pixel count. These settings change rendering workload, not the monitor's resolution or refresh rate.

### Playlist entries: `[[wallpapers]]`

| Key | Default | Meaning / accepted values |
| --- | --- | --- |
| `name` | required | Unique, nonempty selector used by `active` and `set` |
| `type` | required | `"shader"`, `"image"`, or `"video"` |
| `source` | required | Existing local file; relative to this TOML file |
| `enabled` | `true` | Disabled entries are skipped and may reference a missing file |
| `period` | `"any"` | `"any"`, `"day"`, or `"night"` |
| `speed` | `1.0` | Shader clock multiplier, finite `0..10`; `0` freezes shader time |
| `brightness` | `1.0` | Shader/image brightness, finite `0.05..2.0` |
| `fps` | unset | Optional presentation cap, integer `1..240` |
| `render_scale` | unset | Optional rendering scale cap, `0.1..1.0` |
| `stabilize` | `false` | GPU spatial/temporal filter for shaders |
| `note` | unset | Optional description returned by `list` |

The active entry must exist and be enabled. Shader speed/brightness, day/night palette and stabilization do not change video playback speed/colors. Images use brightness but do not animate. Lower video presentation FPS does not necessarily lower decoder FPS; codec/driver behavior still matters.

### Rotation: `[rotation]`

| Key | Default | Meaning |
| --- | --- | --- |
| `enabled` | `false` | Rotate through enabled entries eligible for the current period |
| `interval_seconds` | `600` | Positive interval; `600` is ten minutes |
| `defer_while_paused` | `true` | Wait while all outputs are inactive; advance once after resuming |

Selection follows configuration order. `next`/`prev` wrap and skip synchronous renderer failures. Explicit `set NAME` can choose an enabled entry outside its period. Automatic selection returns to the eligible pool. No rapid catch-up occurs after a long pause.

### Day/night: `[day_night]`

| Key | Default | Meaning |
| --- | --- | --- |
| `enabled` | `false` | Automatic period selection |
| `source` | `"noctalia"` | `"noctalia"` or `"schedule"` |
| `day_start` | `"07:00"` | Local clock time; hour `0..23`, minute `0..59` |
| `night_start` | `"20:00"` | Different from `day_start`; schedules may cross midnight |
| `day_brightness` | `1.0` | Day shader/image multiplier, finite `0.05..2.0` |
| `night_brightness` | `0.65` | Night shader/image multiplier, finite `0.05..2.0` |

When automatic day/night is enabled, each period needs at least one eligible enabled entry; an `any` entry satisfies both. Noctalia's resolved light/dark mode is queried at most once every 30 seconds with a two-second subprocess timeout. The local schedule is the fallback when the query is unavailable. Schedule mode does not query Noctalia. This is a clock schedule, not an astronomical sunrise calculation.

The applied shader/image brightness is the entry multiplier times the resolved period multiplier. A manual `mode day/night` override takes priority; `mode auto` removes it. These commands do not change Noctalia's global theme.

## Shader interface

Use a self-contained GLSL 330 core fragment shader:

```glsl
#version 330 core
in vec2 v_uv;
out vec4 fragColor;
uniform float u_time;
uniform vec2 u_resolution;
uniform float u_night;

void main() {
    float wave = 0.5 + 0.5 * sin(v_uv.x * 8.0 + u_time);
    vec3 day = vec3(0.1, 0.5, 0.8) * wave;
    fragColor = vec4(mix(day, day * 0.4, u_night), 1.0);
}
```

| Input | Meaning |
| --- | --- |
| `v_uv` | Interpolated UV coordinates, vertically flipped by the supplied vertex shader |
| `u_time` | Pause-aware animation time in seconds, already multiplied by entry `speed` |
| `u_resolution` | Current rendering buffer size, not necessarily native screen resolution |
| `u_night` | `0.0` for non-night, `1.0` for night |
| `fragColor` | Required fragment output name |

The loader wraps the literal `void main(` entry point to apply `nmb_brightness`. Keep that spelling and reserve the injected `nmb_brightness`/`nmb_main` names. Shadertoy `mainImage`, `iTime`, `iResolution`, buffers and texture channels are not imported automatically; port them to this interface. General external shader texture-channel binding is not exposed by the configuration.

`stabilize = true` adds a GPU-only 3×3 spatial filter, bounded bright outliers and neighborhood-clamped temporal history. It softens fine detail to reduce flicker. History resets on renderer/size/brightness/period changes. It requires additional GPU textures and passes; it is optional and does not apply to images or video.

## Images and video

Image support is compiled for PNG, JPEG and WebP. Images are aspect-filled and redraw only when needed.

Video uses libmpv's OpenGL render API on the actual Wayland display, loops silently, and disables audio, subtitles, mpv configuration, on-screen controls and default input bindings. It is loaded dynamically, so shader/image-only installations need no libmpv. Supported codecs and hardware decoding depend on the installed mpv build and GPU driver.

`hwdec = "vulkan"` is a decoder option, not a Vulkan renderer. The rendering backend remains EGL/OpenGL. Hardware decoding does not guarantee a zero-copy path for every codec/driver. Decoder status is reported through IPC; with `require_hardware_video = true`, unsupported hardware decoding causes that video to fail/pause instead of silently accepting software decoding.

## Scheduling, power and desktop integration

- Frame callbacks and phase-anchored monotonic deadlines avoid a busy render loop. Missed frame slots are skipped rather than rendered in bursts.
- Animation freezes while all outputs are paused and limits time jumps after stalls.
- Battery status is sampled every 30 seconds from sysfs; only a `Battery` supply reporting `Discharging` activates the battery profile.
- Idle suspension depends on the compositor's idle-notify protocol.
- Hidden suspension is inferred from outstanding frame callbacks. It is a visibility heuristic, not universal window-occlusion detection.
- Fullscreen suspension is inferred from niri's active-workspace tile dimensions via `NIRI_SOCKET`; it is unavailable without that integration.
- Output creation/removal, integer scaling and optional fractional-scale/viewport protocols are handled by the background runtime. Physical multi-monitor hotplug and other GPU vendors still require testing on their respective hardware.

## IPC and environment

| Variable | Use |
| --- | --- |
| `WAYLAND_DISPLAY` | Wayland socket selection |
| `XDG_RUNTIME_DIR` | Wayland/IPC runtime directory; required |
| `MOTION_RENDER_SOCKET` | IPC socket filename; default `niri-motion-background.sock` |
| `NIRI_SOCKET` | Optional niri event-stream integration |
| `RUST_LOG` | Logging filter when the consuming application initializes env_logger |

IPC is newline-delimited commands with JSON replies. `ipc::send` writes a command and prints the reply. The listener socket is mode `0600`; a nonblocking lock rejects another instance using the same name. Use a distinct socket filename for independent background clients. The login supervisor automatically chooses `motion-login-<pid>.sock`.

Commands are `status`, `list`, `set NAME`, `next`, `prev`, `pause`, `resume`, `toggle`, `reload`, `quit`, `mode auto|day|night`, and `auto-switch on|off|toggle`. See the [wallpaper CLI reference](../niri-motion-background/README.md#command-line-reference) for behavior and examples. Runtime commands do not persist settings to TOML.

## Validation and licensing

Unit tests cover configuration, selection, scheduling and niri layout tracking. The sibling wallpaper project contains the real EGL stabilization regression and live shader/image/video smoke test. The login project contains nested Wayland and mock-greetd tests. See their READMEs for prerequisites and commands.

Development validation used Intel Arc B390 with Mesa and VAAPI. Functional pause/FPS checks are not whole-system power measurements or battery-life guarantees.

Library code is [MIT-licensed](LICENSE). Reusing this library does not relicense shader assets: consult the wallpaper project's [THIRD_PARTY.md](../niri-motion-background/THIRD_PARTY.md). The derived login application has its own GPL license.
