# Arch/CachyOS: Motion Login with greetd

> **All AI — a personal experiment. Use at your own risk — agent in YOLO mode.**

For all four checkouts, dependencies and the build order, see the [project-set guide](../../../README.md) in the source tree. In the installed package, consult `/usr/share/doc/motion-login/README.md` for the greeter's own instructions.

The package installs the greeter, shaders, configurations and a drop-in for `greetd.service`. It provides `greetd-greeter`. Motion Render is compiled into the executable, so there is no separate library package. Package installation does not activate/stop services or remove SDDM.

## Build a package and install it

Keep sibling `motion-render` and `niri-motion-background` source folders beside `motion-login`. The latter supplies shader/license assets; its daemon does not need to be running. Install build tools first because this helper invokes `makepkg --nodeps`:

```sh
sudo pacman -S --needed base-devel rust pkgconf python wayland libglvnd libxkbcommon niri
cd ~/workspaces/linux/motion-desktop/motion-login
./packaging/arch/build-package.sh
sudo pacman -U ./dist/arch/motion-login-0.3.0-14-x86_64.pkg.tar.zst
```

The helper snapshots both Rust projects and required shader assets into a checksummed archive, then runs makepkg's build/check steps. Cargo downloads registry crates as needed; no access to the original home-directory paths is required after creating the archive. `pacman -U` resolves declared runtime dependencies, including greetd and fprintd.

Package contents include:

- `/usr/bin/motion-login`
- `/usr/share/motion-login/shaders/`
- `/etc/motion-login/login.toml`: colors, UI scale, glitch settings and `niri-session`
- `/etc/motion-login/background.toml`: shader selection through `active`
- `/etc/motion-login/greetd.toml`: greetd configuration and greeter user
- `/etc/motion-login/greeter.kdl`: dedicated niri compositor for the login screen
- `/etc/pam.d/motion-login`: authentication policy
- `/usr/lib/systemd/system/greetd.service.d/motion-login.conf`

The configuration files use pacman's backup handling; inspect `.pacnew` files on upgrades. `/etc/greetd/config.toml` is not replaced. The greetd package supplies its user, base PAM configuration and service.

Before switching:

```sh
/usr/bin/motion-login --check --config /etc/motion-login/login.toml --background /etc/motion-login/background.toml
niri validate --config /etc/motion-login/greeter.kdl
systemctl cat greetd.service
```

## Test before enabling at boot

The commands below assume SDDM is your existing display manager; adapt them if it differs. Log out normally, then switch to a text console with Ctrl+Alt+F3 and log in there. Keep SDDM enabled for the next boot initially.

```sh
sudo systemctl stop sddm.service
sudo systemctl start greetd.service
```

The new login appears on VT1 (Ctrl+Alt+F1). Check real authentication and whether niri, Noctalia, the keyring and wallpaper service start as expected. Preview and mock tests do not verify real credentials or the actual session handoff.

If login fails, return to Ctrl+Alt+F3:

```sh
sudo systemctl stop greetd.service
sudo systemctl start sddm.service
journalctl -b -u greetd.service
```

## Enable after a successful test

```sh
sudo systemctl disable sddm.service
sudo systemctl enable --force greetd.service
```

Do not add `--now` inside a running desktop session. The `display-manager.service` alias now points to greetd. SDDM can remain installed for rollback.

To restore the old setup, log out and use a text console:

```sh
sudo systemctl stop greetd.service
sudo systemctl disable greetd.service
sudo systemctl enable --force sddm.service
sudo systemctl start sddm.service
```

Removing motion-login leaves greetd installed and removes the drop-in, so greetd falls back to its own configuration. Enable your intended display manager before removing this package.

## Fingerprint authentication

The package supplies `/etc/pam.d/motion-login`. In `/etc/motion-login/greetd.toml`, the service selectors must be:

```toml
[general]
service = "motion-login"
[default_session]
service = "greetd"
```

Keep the other command/user settings. Merge these selectors from `.pacnew` files when needed. Enroll and verify a finger with `fprintd-enroll` and `fprintd-verify`. After username submission, a scan runs for up to ten seconds, then password authentication remains available. Cancellation can wait for the current PAM call; a late scan response must not start a session. For password-only authentication, set `[general] service = "greetd"`.

Application labels are English. PAM/greetd/fprintd messages can follow the system locale and are displayed verbatim.
