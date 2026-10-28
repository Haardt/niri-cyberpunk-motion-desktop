# Motion desktop experiments

A cyberpunk-inspired look and feel for a Wayland/niri desktop: custom login and lock screens, animated wallpapers, and a fork of the **Noctalia** desktop shell with glitch animations.

> **All AI — a personal experiment.** This is a personal test of building and configuring a Linux desktop with coding agents. Treat the generated code and configuration as experimental. Existing upstream code, fonts, shaders and their licenses retain their original attribution.
>
> **The year of the Linux desktop: coding agents let you configure everything. BUT: use at your own risk — agent in YOLO mode.**

## Projects and dependencies

| Project | Role | Source dependencies | Runtime integration |
| --- | --- | --- | --- |
| [motion-render](motion-render/README.md) | Shared Rust EGL/OpenGL, shader/image/video and Wayland library | Cargo crates in its lockfile | Compiled into consumers; no separate service, binary or Arch package |
| [niri-motion-background](niri-motion-background/README.md) | Desktop wallpaper daemon, CLI and shader collection | Sibling `../motion-render` through a Cargo path dependency | Wayland compositor, EGL/OpenGL; optional niri/Noctalia integration and libmpv video |
| [motion-login](motion-login/README.md) | greetd login UI and background supervisor | Sibling `../motion-render`; configuration and packaging also use shader/license assets from `../niri-motion-background` | greetd, dedicated niri compositor, PAM/fprintd; optional libmpv |
| [motion-lock](motion-lock/README.md) | Wayland session locker with password/fingerprint unlock | Its own `vendor/motion-render` snapshot and copied login drawing code; no sibling Cargo dependency | `ext-session-lock-v1`, PAM; optional fprintd; wallpaper config/shaders unless an explicit shader is configured |

`motion-lock` is deliberately independent at build time. Editing the sibling renderer or login UI does **not** update the locker. Review and explicitly port changes into its snapshot/copied UI when needed. At runtime its default shader discovery reads `~/.config/niri-motion-background/config.toml` and queries the running wallpaper daemon when available. A fixed `shader` path removes that runtime dependency.

Noctalia, niri, greetd, fprintd and mpv are external projects. The local `noctalia-cyberpunk` directory is a separate shell customization, not a fifth build dependency of these four projects. Wallpaper integration can call an already installed Noctalia, but building this set does not require that checkout.

## One repository, one checkout

This monorepo keeps the four projects as sibling directories:

```text
motion-desktop/
├── README.md
├── motion-render/
├── niri-motion-background/
├── motion-login/
└── motion-lock/
```

The repository is hosted at [Haardt/niri-cyberpunk-motion-desktop](https://github.com/Haardt/niri-cyberpunk-motion-desktop). Clone all four projects together:

```sh
mkdir -p ~/workspaces/linux
cd ~/workspaces/linux
git clone git@github.com:Haardt/niri-cyberpunk-motion-desktop.git motion-desktop
cd motion-desktop
```

For the existing local copy, use `cd ~/workspaces/linux/motion-desktop`. The `origin` remote points to the GitHub repository.

The four original source folders were copied with `cp -a` and remain unchanged at `~/workspaces/linux/{motion-render,niri-motion-background,motion-login,motion-lock}`. The new directories are independent copies, not links back to those originals. Use the monorepo for future edits to avoid diverging source copies. Build products and caches were copied locally as requested, but `.gitignore` excludes them from Git.

Each Rust project retains its own `Cargo.toml` and `Cargo.lock`; this is a Git monorepo, not a new Cargo workspace. Existing commands and packaging scripts continue to use sibling paths. Cargo obtains registry crates during the first build, so a fresh checkout normally needs network access. Commit related changes together to keep the sibling renderer, applications and shader assets compatible.


## Arch/CachyOS prerequisites

Install build tools and the native libraries used across the four projects:

```sh
sudo pacman -S --needed base-devel rust clang pkgconf python wayland libglvnd mesa libxkbcommon pam niri
```

`rust` supplies Cargo. A current stable rustup toolchain also works, but do not install conflicting Rust providers together. The login packaging helper uses `makepkg --nodeps`, so it is particularly important to install the build tools first. The locker and wallpaper helpers can use the same mode when Cargo comes from a user-owned rustup installation. Run package builds as your regular user, not root.

Optional/production runtime components:

```sh
# Required for the production login screen (declared by its Arch package):
sudo pacman -S --needed greetd fprintd dbus xkeyboard-config
# Optional video backgrounds:
sudo pacman -S --needed mpv
# Optional session-lock integration with logind/sleep:
sudo pacman -S --needed swayidle
```

Fingerprint authentication additionally requires supported hardware and an enrolled finger. The shader backend needs a working EGL/OpenGL 3.3 driver; installing Mesa alone does not configure every GPU.

## Build order and checks

From `~/workspaces/linux/motion-desktop`, a useful verification order is:

```sh
cargo test --locked --manifest-path motion-render/Cargo.toml
cargo build --release --locked --manifest-path niri-motion-background/Cargo.toml
cargo test --locked --manifest-path niri-motion-background/Cargo.toml
cargo build --release --locked --manifest-path motion-login/Cargo.toml
cargo test --locked --manifest-path motion-login/Cargo.toml
cargo build --release --locked --manifest-path motion-lock/Cargo.toml
cargo test --locked --manifest-path motion-lock/Cargo.toml
```

This is a review order, not a requirement to install a renderer first. Cargo automatically compiles each path dependency. Login and wallpaper can build independently once their sibling sources exist; the locker can build on its own because its renderer is vendored. The wallpaper daemon does not have to be running to build the login screen.

Use `CARGO_BUILD_JOBS=4` to limit parallel compilation if needed. GPU, nested-compositor and authentication tests have additional requirements described in the project READMEs; unit/configuration tests alone do not establish that real login/unlock or suspend/resume works on your hardware.

## Build Arch packages, then install with pacman

There are three executable packages. `motion-render` is compiled into them and does not need a standalone package. These commands use the versions currently declared in the package recipes; when changing versions/releases, use the corresponding newly generated filename. `makepkg --packagelist` in the package build directory lists the output paths.

```sh
cd ~/workspaces/linux/motion-desktop/niri-motion-background
./packaging/arch/build-package.sh
sudo pacman -U ./dist/arch/niri-motion-background-0.1.0-1-x86_64.pkg.tar.zst

cd ~/workspaces/linux/motion-desktop/motion-login
./packaging/arch/build-package.sh
sudo pacman -U ./dist/arch/motion-login-0.3.0-14-x86_64.pkg.tar.zst

cd ~/workspaces/linux/motion-desktop/motion-lock
./packaging/build-package.sh
sudo pacman -U ./motion-lock-0.2.0-1-x86_64.pkg.tar.zst
```

The helpers create source archives and checksums, then run `makepkg` including its checks. The login archive includes the sibling renderer and wallpaper shader/licenses; the wallpaper archive includes its sibling renderer; the locker archive includes only its own project and vendored sources. Do not invoke the template wallpaper/login PKGBUILDs directly: their source checksum placeholders are filled by the helpers. Installing with `pacman -U` resolves declared repository dependencies. Package installation does not activate greetd or the wallpaper service, or replace your existing session-lock setup.

## Configure and activate

1. **Wallpaper:** follow the [package setup](niri-motion-background/README.md#arch-package-installation) to copy the initial per-user config/shaders, then enable its user service from your Wayland session. Existing user-installed binaries and user service overrides take precedence over packaged versions; review them when migrating from `install.sh`.
2. **Login:** preview with `motion-login/preview.sh`, then follow the [greetd activation and rollback guide](motion-login/packaging/arch/INSTALL.md). Switching display managers is a separate step performed from a TTY after logging out. Installing a package does not switch the running desktop.
3. **Locker:** run `motion-lock --check` and `motion-lock --preview`, then configure [shortcuts, idle/menu and sleep integration](motion-lock/README.md#desktop-integration). Test real unlock and resume yourself before relying on it.

Noctalia's internal lock screen and an external locker must not compete for the same lock. The [locker README](motion-lock/README.md#current-workstation-integration) records the separate integration currently installed on this workstation. That local setup is not automatically shipped by any of these packages.
