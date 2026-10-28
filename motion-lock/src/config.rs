use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub shader: Option<PathBuf>,
    pub pam_service: String,
    pub fingerprint: bool,
    pub fingerprint_pam_service: String,
    pub title: String,
    pub background: crate::color::Color,
    pub headline: crate::color::Color,
    pub prompt: crate::color::Color,
    pub prompt_err: crate::color::Color,
    pub ui_scale: f32,
    pub fps: u32,
    pub battery_fps: u32,
    pub shader_speed: f32,
    pub brightness: f32,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            shader: None,
            pam_service: "motion-lock".into(),
            fingerprint: true,
            fingerprint_pam_service: "motion-lock-fingerprint".into(),
            title: "SESSION LOCK".into(),
            background: crate::color::Color::new(0.03, 0.05, 0.08, 1.),
            headline: crate::color::Color::new(0.98, 0.86, 0.41, 1.),
            prompt: crate::color::Color::new(0.21, 0.90, 0.96, 1.),
            prompt_err: crate::color::Color::new(1., 0.38, 0.78, 1.),
            ui_scale: 1.,
            fps: 20,
            battery_fps: 12,
            shader_speed: 0.1,
            brightness: 1.,
        }
    }
}
impl Config {
    pub fn load(path: Option<&Path>) -> Result<Self> {
        if let Some(explicit) = path {
            if !explicit.is_file() {
                bail!("configuration {} does not exist", explicit.display());
            }
        }
        let default = home().join(".config/motion-lock/config.toml");
        let path = path.unwrap_or(&default);
        let mut c: Self = if path.exists() {
            toml::from_str(&std::fs::read_to_string(path)?)?
        } else {
            Self::default()
        };
        if c.fps == 0
            || c.fps > 60
            || c.battery_fps == 0
            || c.battery_fps > 60
            || !c.ui_scale.is_finite()
            || !(0.5..=2.5).contains(&c.ui_scale)
            || !c.brightness.is_finite()
            || !(0.1..=1.).contains(&c.brightness)
            || !c.shader_speed.is_finite()
            || !(0.001..=10.).contains(&c.shader_speed)
        {
            bail!("invalid FPS, brightness, speed or UI scale");
        }
        if let Some(p) = &c.shader {
            if p.is_relative() {
                c.shader = Some(path.parent().unwrap().join(p));
            }
        }
        Ok(c)
    }
    pub fn shader_path(&mut self) -> Result<PathBuf> {
        if let Some(p) = &self.shader {
            return p.canonicalize().context("configured shader not found");
        }
        let source = home().join(".config/niri-motion-background/config.toml");
        let config = motion_render::config::Config::load(&source)?;
        let active = current_wallpaper().unwrap_or(config.active.clone());
        let wp = config
            .wallpapers
            .iter()
            .find(|w| {
                w.name == active && w.enabled && w.kind == motion_render::config::Kind::Shader
            })
            .or_else(|| {
                config
                    .wallpapers
                    .iter()
                    .find(|w| w.enabled && w.kind == motion_render::config::Kind::Shader)
            })
            .context("no enabled shader in wallpaper configuration")?;
        self.shader_speed = wp.speed;
        Ok(wp.source.clone())
    }
}
pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}
fn current_wallpaper() -> Option<String> {
    let path =
        PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR")?).join("niri-motion-background.sock");
    let mut s = UnixStream::connect(path).ok()?;
    s.set_read_timeout(Some(Duration::from_millis(300))).ok()?;
    s.set_write_timeout(Some(Duration::from_millis(300))).ok()?;
    s.write_all(b"status\n").ok()?;
    let mut buf = String::new();
    s.take(16384).read_to_string(&mut buf).ok()?;
    serde_json::from_str::<serde_json::Value>(&buf)
        .ok()?
        .get("wallpaper")?
        .as_str()
        .map(str::to_owned)
}
