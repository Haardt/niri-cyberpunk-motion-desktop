# Cyberpunk prototype

Local branch: `cyberpunk/prototype`, based on upstream `v5.3.0`.
The system package `/usr/bin/noctalia` remains available as a fallback.
The preview is isolated; the live installation uses the normal user configuration.

## Build and run

```sh
meson setup build-cyberpunk --buildtype=debugoptimized -Dtests=enabled
meson compile -C build-cyberpunk -j 6 noctalia
./tools/cyberpunk-preview
```

The preview opens a **nested Niri desktop in a window**, on its own Wayland
display and private D-Bus session. Never add `--session` to this Niri command.
It uses `cyberpunk/config/` and keeps its state under `build-cyberpunk/`.
Authentication, notification ownership, lock screen, theme template application,
clipboard integration, and automatic plugin updates are disabled for the preview.

- Click the clock or press **Alt+C** to open/close the panel and replay its opening effect.
- **Alt+Space** opens the launcher.
- **Alt+Shift+Q** closes the nested desktop.
- `./tools/cyberpunk-preview --baseline` runs the same visual theme without the renderer effect.

## Effect

`NOCTALIA_CYBERPUNK=1` opts transient panels into a 320 ms opening burst
and individual bar capsules into occasional 1.6–2.2 second ambient sweeps. Three thin horizontal strips are re-rendered
with a bounded horizontal offset, accompanied by cyan/red signal accents.
Small yellow/cyan corner markers remain when settled. Scene layout and hit
regions do not move. Wallpaper, lock screen, screenshots and unrelated surfaces
are not opted into this effect.

The prototype uses the existing GLES renderer and scissor clipping rather than
adding an offscreen framebuffer or a new full-screen shader pass. It does not
perform true RGB-channel separation. The phase saturates after one burst;
each burst stops scheduling frames once settled. Bar capsules use a separate
one-shot timer with random pauses of 6–12 seconds. Each timer firing selects
one currently laid-out widget (a member inside grouped islands); attached popups and hidden bars suppress the burst; pointer hover no longer
suppresses it. Scissor clipping confines displaced strips and accent bands
to that element, leaving its neighbors untouched. No whole-bar opening burst
is used. The timer is owned by the bar instance and canceled on destruction. Existing widget sampling and ordinary shell animations still run.

The palette and angular TOML theme are independent of the C++ effect. The
shared inspiration is the existing motion-login/motion-lock burst rhythm, with
a restrained intensity for everyday shell use.

## Validation

```sh
meson test -C build-cyberpunk cyberpunk_effect --print-errorlogs
NOCTALIA_CONFIG_HOME="$PWD/cyberpunk/config" \
  NOCTALIA_STATE_HOME="$PWD/build-cyberpunk/preview-state" \
  ./build-cyberpunk/noctalia config validate
niri validate -c cyberpunk/niri.kdl
```

The timing test covers bounded strength and settling after delayed frames,
monotonic downward strip travel and smooth horizontal drift limited to 10 px over multiple variations.
`NOCTALIA_IDLE_PROFILE=1` is enabled by the preview launcher so its logs can be
used to inspect frame scheduling after the initial burst.

## Current layout and angular styling

`bar-cyberpunk.toml` and `panels-solid.toml` snapshot the current desktop layout
(24 October 2026): workspace/media/capture widgets, clock, resource and status
islands, tray, and floating opaque panels. Colors follow the preview palette.
The portable preview disables plugins. The original workstation prototype used
a local recorder-plugin source; that machine-specific path is not required here.

With `NOCTALIA_CYBERPUNK=1`, eligible rectangle fills and borders use clipped
top-left and bottom-right corners. Round icon buttons remain round. Larger
bar capsules carry small `///` marks in their lower inset. Dark olive surfaces,
yellow selection fills and outlines, cyan readings and red alerts follow the
reference direction. These contours are implemented in the existing rectangle
shader, without changing widget layout.

Tooltips replay a scoped 1.6–2.2 second downward sweep every 2400 ms while visible. Their timer
stops on dismissal, forced teardown and destruction. Scoped effects use wider
slices, 4–10 logical pixels of smooth displacement and brighter signal bands;
neighboring widgets remain outside the scissor region.

Scoped glitches travel from top to bottom through the element. Horizontal drift
is continuous and slow; strip positions and accent marks do not jump randomly.

Each burst derives a fixed variation from its start time. Strip count (2–4),
width, drift direction, amplitude, duration and signal-band placement vary
between passes, while each individual pass remains smooth and downward.

## Live installation

The stripped binary and matching assets are installed in
`~/.local/libexec/noctalia-cyberpunk/`. `~/.local/bin/noctalia` opts in to
the effect and uses those assets. Niri autostart explicitly calls this wrapper
so login does not depend on the compositor's PATH. Normal shell services,
plugins and current widget layout continue to use the user's own settings.
`cyberpunk-live.toml` selects the angular panel styling and monospace font;
the custom preview palette is installed and selected through Noctalia IPC.
The live wrapper hides the development DEBUG indicator.

Backups and an executable `restore.py` are saved under
`~/.local/state/noctalia-cyberpunk/backups/<timestamp>/`. Restoration stops
the custom shell, restores the touched configuration files and starts the
system package. `/usr/bin/noctalia` is never overwritten.
