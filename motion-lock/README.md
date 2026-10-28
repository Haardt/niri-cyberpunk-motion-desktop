# Motion Lock

> **All AI — a personal experiment.** Built and configured with coding agents as a personal test. Experimental code: use at your own risk. Upstream code, fonts and shaders retain their original attribution and licenses.
>
> **The year of the Linux desktop: coding agents let you configure everything. BUT: use at your own risk — agent in YOLO mode.**

A Rust Wayland session locker with animated shaders from Motion Render and a scalable interface derived from Motion Login. It uses `ext-session-lock-v1`, creates a lock surface per output, and supports password and fingerprint authentication.

## Projects and dependencies

See the [complete checkout, build-order and Arch installation guide](../README.md) for all four projects.

Unlike the wallpaper and greeter, this project uses `vendor/motion-render`, not the sibling renderer directory. Its drawing/UI code is copied from Motion Login and maintained here. It builds independently of the other checkouts; updating them does not update this snapshot or interface. See `THIRD-PARTY.txt` for provenance.

Build requirements: current stable Rust/Cargo, a C linker, Clang/libclang for PAM bindings, pkg-config, PAM, Wayland/EGL/OpenGL and libxkbcommon. Runtime requires a compositor with `ext-session-lock-v1`, a working OpenGL 3.3 driver and the installed PAM policies. `fprintd` and an enrolled finger are optional. Noctalia and greetd are not locker dependencies.

By default, shader discovery reads `~/.config/niri-motion-background/config.toml` and queries the running wallpaper daemon for its current selection. If the current wallpaper is an image/video, it chooses an enabled shader from that config. A fixed `shader` setting or `--shader FILE` bypasses discovery, so a wallpaper installation is optional. The background is rendered live, not captured from the desktop.

## Build and install on Arch/CachyOS

Install the native build dependencies, then build as your regular user:

```sh
sudo pacman -S --needed base-devel rust clang pkgconf pam wayland libglvnd mesa libxkbcommon python
cd ~/workspaces/linux/motion-desktop/motion-lock
cargo build --release --locked -j 4
cargo test --locked -j 4
CARGO_BUILD_JOBS=4 ./packaging/build-package.sh
sudo pacman -U ./motion-lock-0.2.0-1-x86_64.pkg.tar.zst
```

The helper snapshots only this project, including `vendor/`, fills the source archive's SHA-256 checksum and runs `makepkg` with checks. It defaults to 11 Cargo jobs; set `CARGO_BUILD_JOBS` to suit your machine. When Cargo comes from rustup outside pacman's database, it checks for Clang and the required pkg-config entries and uses `makepkg --nodeps`; install all actual dependencies first.

| Installed path | Purpose |
| --- | --- |
| `/usr/bin/motion-lock` | Locker and preview executable |
| `/etc/pam.d/motion-lock` | Password and account checks through `system-auth` |
| `/etc/pam.d/motion-lock-fingerprint` | Separate fingerprint and account checks |
| `/usr/share/applications/motion-lock-preview.desktop` | Preview launcher |
| `/usr/share/doc/motion-lock/` | README and example configuration |

PAM policies use pacman's backup handling; review `.pacnew` files when upgrading. Package installation does not change niri shortcuts, Noctalia settings or suspend handling. Rebuild the package to update installed UI text and documentation; editing sources alone leaves `/usr/bin/motion-lock` unchanged.

## Check, preview and lock

```sh
# Check configuration, shader selection and PAM availability without locking:
motion-lock --check
# Preview in a normal window, without authentication:
motion-lock --preview --preview-seconds 20
# Lock the current session:
motion-lock --lock
```

Preview never locks, authenticates or starts the fingerprint scanner. Use synthetic text when testing it; Escape closes it. `--preview-seconds` is accepted only in preview mode. Real locking has no grace period or timed unlock.

| Input | Action while locked |
| --- | --- |
| Enter or Unlock button | Authenticate the supplied password |
| Backspace | Delete the last character |
| Ctrl+U | Clear the input |
| Escape | Clear the input; keep the session locked |
| Enrolled finger | Authenticate once the scanner is active |

## Configuration and shaders

Optional configuration: `~/.config/motion-lock/config.toml`. Without it the built-in defaults apply. Copy the example only if you do not already have a configuration:

```sh
mkdir -p ~/.config/motion-lock
cp -n /usr/share/doc/motion-lock/config.example.toml ~/.config/motion-lock/config.toml
```

```toml
pam_service = "motion-lock"
fingerprint = true
fingerprint_pam_service = "motion-lock-fingerprint"
ui_scale = 1.15
fps = 20
battery_fps = 12
brightness = 1.0
shader_speed = 0.1
# Use a real absolute path to bypass wallpaper discovery:
# shader = "/absolute/path/to/plasma.frag"
```

| Setting | Meaning |
| --- | --- |
| `ui_scale` | Panel size independently of display scaling; default 1.0 |
| `fps` / `battery_fps` | Frame-rate cap on AC / battery; default 20 / 12 |
| `brightness` | Shader brightness; default 1.0 |
| `shader` | Optional fixed shader path; relative paths resolve against the config directory |
| `shader_speed` | Speed for a fixed shader; discovery instead uses the wallpaper entry's speed |
| `fingerprint` | Enable the scanner; default true |
| `pam_service` | Password authentication policy name |
| `fingerprint_pam_service` | Independent scanner policy name |
| `title` | Panel title; default `SESSION LOCK` |
| `background`, `headline`, `prompt`, `prompt_err` | Colors; format defined in `src/color.rs` |

Explicit `--config FILE` and `--shader FILE` overrides are also supported. The interface shows local time, an ISO-style date and ISO calendar week in English. The shader renders at half width/height and is GPU-upscaled; UI drawing remains at native resolution. Frame caps and Wayland frame callbacks limit rendering work.

## Fingerprint and PAM

```sh
sudo pacman -S --needed fprintd
fprintd-list "$USER"
fprintd-enroll
# Optional standalone verification; no other application may hold the sensor:
fprintd-verify
```

The scanner starts only after the compositor confirms the lock. It runs in a separate PAM subprocess; password verification runs in a worker thread, so password input remains available while scanning. Both methods require authentication **and** account checks. The fingerprint policy includes `pam_faillock` and `pam_fprintd.so max-tries=3 timeout=10`.

Failed scans retry after five seconds. A missing fingerprint policy leaves password authentication available. Set `fingerprint = false` to disable the scanner. Unlock/exit stops its subprocess and releases the sensor. Additional password-PAM challenges remain visible. Application-owned strings are English; messages supplied by PAM/fprintd can follow the system locale and are displayed verbatim.

## Desktop integration

For basic manual locking, merge these entries into the existing niri `binds` block, replacing old bindings for the same keys:

```kdl
Mod+Alt+L repeat=false hotkey-overlay-title="Lock Screen: motion-lock" {
    spawn "/usr/bin/motion-lock" "--lock";
}
Ctrl+Alt+L repeat=false hotkey-overlay-title="Lock Screen: motion-lock" {
    spawn "/usr/bin/motion-lock" "--lock";
}
```

`Mod` is normally Super/Windows. Validate and reload:

```sh
niri validate
niri msg action load-config-file
```

Shortcuts alone do not integrate automatic idle locking, logind `Lock` requests, the power menu or suspend/lid close. Configure those separately. With Noctalia, an idle behavior can use `action = "command"` and a session-menu action can override `command`. Disable the internal lock screen when an external locker owns these paths.

For sleep/logind handling, `swayidle -w` supports `lock` and `before-sleep` commands. The sleep helper must return **after the compositor confirms the lock**, rather than waiting for the locker to exit after unlock. Conversely, backgrounding `motion-lock` and returning immediately does not establish that locking succeeded. Do not use the foreground `motion-lock --lock` command directly with `swayidle -w before-sleep`.

### Current workstation integration

As of 2026-10-28, a separate local integration on this CachyOS/niri workstation uses:

| File or component | Responsibility |
| --- | --- |
| `~/.local/bin/session-lock` | Start one locker, wait up to four seconds for `session lock confirmed`; `--suspend` requests sleep after confirmation |
| `~/.config/systemd/user/motion-lock-events.service` | Run swayidle for logind lock and before-sleep events; enabled for the graphical session |
| `~/.local/libexec/motion-lock-integration/` | Locally extracted, signature-verified CachyOS swayidle package and integration notes |
| `~/.config/niri/cfg/keybinds.kdl` | Super+Alt+L and Ctrl+Alt+L call the helper |
| `~/.config/noctalia/config.toml` | Disable internal lock screen; menu Lock and Lock & Suspend call the helper |
| `~/.local/state/noctalia/settings.toml` | Automatic lock at 600 seconds; screen-off remains at 660 seconds |

These files are outside this checkout and are **not installed by the Arch package**. A clean machine needs its own integration. Config syntax, service startup and `motion-lock --check` were checked during setup; real unlock and suspend/resume were not triggered. The four-second helper timeout is an error limit, not a guarantee that sleep will be prevented if locking fails. Test the entire flow on your hardware.

Inspect the local integration:

```sh
systemctl --user status motion-lock-events.service
journalctl --user -u motion-lock-events.service
cat "$XDG_RUNTIME_DIR/motion-lock-integration.log"
```

Pre-integration backups are in `~/.local/state/motion-lock-integration/20261028-110611/`. Restoring Noctalia requires restoring its configuration/idle settings and shortcuts, then disabling the external events service; changing just one shortcut is insufficient.

## Tests, troubleshooting and limits

```sh
cargo test --locked
# Requires niri and a working Wayland session; runs in a separate compositor:
python3 tests/nested-smoke.py
```

The nested smoke test checks rejection of invalid shaders before locking, lock confirmation, rejection of a second locker and continued compositor locking after SIGKILL. It never locks the host desktop. Unit tests cover CLI mode exclusivity, scaling, title sizing and PAM error paths; fingerprint tests cover a missing policy and real rejecting `other` policy, not successful biometric unlock.

- Wrong locker: inspect shortcuts, menu, idle and sleep commands independently.
- Startup failure: run `motion-lock --check`. A missing password PAM policy prevents real locking before the lock request.
- Shader failure: check the reported path. Invalid GLSL is rejected before locking.
- Fingerprint unavailable: check enrollment, PAM policy and sensor ownership; try the password.
- The current source does not provide an IME or on-screen keyboard. Successful password/fingerprint unlock, physical hotplug, fractional scaling and suspend/resume still require hardware testing.

Only successful authentication sends an unlock request. A crash leaves the compositor lock in place; recovery may require a separate TTY/session or a compositor-supported replacement locker. Core dumps are disabled in lock mode, and password buffers are zeroized rather than logged. Copies made inside PAM are outside that guarantee.

Licenses and provenance: `LICENSE`, `THIRD-PARTY.txt`, `vendor/` and `fonts/`.
