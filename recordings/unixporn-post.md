# Title

[niri] Cyberpunk desktop — animated wallpapers and glitching Noctalia

# Details comment

A personal experiment in building and configuring a Linux desktop with coding agents. The year of the Linux desktop: you can configure everything. Use at your own risk — agent in YOLO mode.

- Compositor: niri
- Desktop shell: a customized Noctalia fork with cyberpunk styling and glitch animations
- Terminal and shell: Alacritty and my regular zsh configuration, opened with Super+Enter
- Backgrounds: GPU-rendered GLSL shaders through niri-motion-background
- Login / lock: motion-login (greetd) and motion-lock (Wayland session locking)
- Hardware: Framework laptop; login and lock screens are prepared for fingerprint authentication via fprintd and dedicated PAM policies

The recording shows launcher glitches, a live CPU tooltip, terminals and window resizing, workspaces 4 and 5, login/lock previews, and three animated backgrounds. Authentication is disabled in the previews; the video does not demonstrate a fingerprint login or unlock.

Motion component sources and build instructions: https://github.com/Haardt/niri-cyberpunk-motion-desktop

The customized Noctalia shell is included under `noctalia-cyberpunk/`, with its own Arch package recipe and profile. Upstream code, fonts and shaders retain their original attribution and licenses.
