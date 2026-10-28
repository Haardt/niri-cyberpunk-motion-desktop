use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub fps: u32,
    pub battery_fps: u32,
    pub render_scale: f64,
    pub battery_render_scale: f64,
    pub pause_on_idle: bool,
    pub idle_seconds: u32,
    pub pause_on_battery: bool,
    pub pause_when_hidden: bool,
    pub pause_on_fullscreen: bool,
    pub hidden_timeout_ms: u64,
    pub hwdec: String,
    pub require_hardware_video: bool,
    pub active: String,
    pub wallpapers: Vec<Wallpaper>,
    pub rotation: Rotation,
    pub day_night: DayNight,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Wallpaper {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: Kind,
    pub source: PathBuf,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub period: Period,
    #[serde(default = "one")]
    pub speed: f32,
    #[serde(default = "one")]
    pub brightness: f32,
    #[serde(default)]
    pub fps: Option<u32>,
    #[serde(default)]
    pub render_scale: Option<f64>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub stabilize: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Shader,
    Image,
    Video,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            fps: 24,
            battery_fps: 12,
            render_scale: 0.4,
            battery_render_scale: 0.32,
            pause_on_idle: true,
            idle_seconds: 120,
            pause_on_battery: false,
            pause_when_hidden: true,
            pause_on_fullscreen: true,
            hidden_timeout_ms: 750,
            hwdec: "auto-safe".into(),
            require_hardware_video: false,
            active: "topo".into(),
            wallpapers: vec![],
            rotation: Rotation::default(),
            day_night: DayNight::default(),
        }
    }
}
impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut config: Self = toml::from_str(&text).context("invalid configuration")?;
        config.validate()?;
        let base = path.canonicalize()?.parent().unwrap().to_owned();
        for wp in &mut config.wallpapers {
            if wp.source.is_relative() {
                wp.source = base.join(&wp.source);
            }
            if wp.enabled && !wp.source.is_file() {
                bail!("wallpaper {}: missing {}", wp.name, wp.source.display());
            }
        }
        Ok(config)
    }
    pub fn validate(&self) -> Result<()> {
        if !(1..=240).contains(&self.fps) || !(1..=240).contains(&self.battery_fps) {
            bail!("fps and battery_fps must be between 1 and 240");
        }
        if !(0.1..=1.0).contains(&self.render_scale)
            || !(0.1..=1.0).contains(&self.battery_render_scale)
        {
            bail!("render scales must be between 0.1 and 1.0");
        }
        if self.idle_seconds == 0 || self.hidden_timeout_ms < 100 {
            bail!("idle_seconds must be positive and hidden_timeout_ms >= 100");
        }
        if self.wallpapers.is_empty() {
            bail!("configure at least one wallpaper");
        }
        let mut names = std::collections::HashSet::new();
        for w in &self.wallpapers {
            if w.name.is_empty() || !names.insert(&w.name) {
                bail!("wallpaper names must be nonempty and unique");
            }
        }
        for w in &self.wallpapers {
            if !w.speed.is_finite()
                || !(0.0..=10.0).contains(&w.speed)
                || !w.brightness.is_finite()
                || !(0.05..=2.0).contains(&w.brightness)
            {
                bail!("{}: speed must be 0..10 and brightness 0.05..2", w.name);
            }
            if w.render_scale
                .is_some_and(|scale| !(0.1..=1.0).contains(&scale))
            {
                bail!("{}: render_scale must be 0.1..1", w.name);
            }
            if w.fps.is_some_and(|fps| !(1..=240).contains(&fps)) {
                bail!("{}: fps must be 1..240", w.name);
            }
        }
        if self.rotation.interval_seconds == 0 {
            bail!("rotation.interval_seconds must be positive");
        }
        let day = parse_clock(&self.day_night.day_start)?;
        let night = parse_clock(&self.day_night.night_start)?;
        if !self.day_night.day_brightness.is_finite()
            || !self.day_night.night_brightness.is_finite()
            || !(0.05..=2.0).contains(&self.day_night.day_brightness)
            || !(0.05..=2.0).contains(&self.day_night.night_brightness)
        {
            bail!("day/night brightness must be 0.05..2");
        }
        if day == night {
            bail!("day_start and night_start must differ");
        }
        if self.day_night.enabled {
            for period in [Period::Day, Period::Night] {
                if self.eligible(period).is_empty() {
                    bail!("configure an enabled wallpaper for {period:?} (or period=any)");
                }
            }
        }
        if !names.contains(&self.active) {
            bail!("active wallpaper '{}' not found", self.active);
        }
        if !self.wallpapers[self.index()].enabled {
            bail!("active wallpaper must be enabled");
        }
        if !["auto-safe", "auto", "no", "vaapi", "nvdec", "vulkan"].contains(&self.hwdec.as_str()) {
            bail!("unsupported hwdec mode");
        }
        if self.require_hardware_video && self.hwdec == "no" {
            bail!("require_hardware_video conflicts with hwdec=no");
        }
        Ok(())
    }
    pub fn index(&self) -> usize {
        self.wallpapers
            .iter()
            .position(|w| w.name == self.active)
            .unwrap()
    }
    pub fn eligible(&self, period: Period) -> Vec<usize> {
        self.wallpapers
            .iter()
            .enumerate()
            .filter(|(_, w)| {
                w.enabled
                    && (period == Period::Any || w.period == Period::Any || w.period == period)
            })
            .map(|(i, _)| i)
            .collect()
    }
    pub fn frame_rate(&self, battery: bool) -> u32 {
        self.wallpapers[self.index()]
            .fps
            .unwrap_or(self.fps)
            .min(self.fps(battery))
    }
    pub fn fps(&self, battery: bool) -> u32 {
        if battery {
            self.battery_fps.min(self.fps)
        } else {
            self.fps
        }
    }
    pub fn effective_scale(&self, battery: bool) -> f64 {
        self.scale(battery)
            .min(self.wallpapers[self.index()].render_scale.unwrap_or(1.))
    }
    pub fn scale(&self, battery: bool) -> f64 {
        if battery {
            self.battery_render_scale.min(self.render_scale)
        } else {
            self.render_scale
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> Config {
        Config {
            wallpapers: vec![Wallpaper {
                name: "topo".into(),
                kind: Kind::Shader,
                source: "topo.frag".into(),
                enabled: true,
                period: Period::Any,
                speed: 1.,
                brightness: 1.,
                fps: None,
                render_scale: None,
                note: None,
                stabilize: false,
            }],
            ..Config::default()
        }
    }
    #[test]
    fn rejects_invalid_power_settings() {
        for fps in [0, 241] {
            let mut c = valid();
            c.fps = fps;
            assert!(c.validate().is_err());
        }
        let mut c = valid();
        c.render_scale = f64::NAN;
        assert!(c.validate().is_err());
        let mut c = valid();
        c.wallpapers.push(c.wallpapers[0].clone());
        assert!(c.validate().is_err());
    }
    #[test]
    fn battery_never_increases_work() {
        let mut c = valid();
        c.battery_fps = 60;
        c.battery_render_scale = 1.;
        assert_eq!(c.fps(true), 24);
        assert_eq!(c.scale(true), 0.4);
        c.wallpapers[0].render_scale = Some(0.2);
        assert_eq!(c.effective_scale(true), 0.2);
        c.wallpapers[0].render_scale = Some(1.0);
        assert_eq!(c.effective_scale(true), 0.4);
        c.wallpapers[0].render_scale = Some(f64::NAN);
        assert!(c.validate().is_err());
    }
    #[test]
    fn rejects_unknown_keys() {
        assert!(toml::from_str::<Config>("fpz = 30").is_err());
    }
}

fn yes() -> bool {
    true
}
fn one() -> f32 {
    1.0
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    #[default]
    Any,
    Day,
    Night,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Rotation {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub defer_while_paused: bool,
}
impl Default for Rotation {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: 600,
            defer_while_paused: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSource {
    #[default]
    Noctalia,
    Schedule,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DayNight {
    pub enabled: bool,
    pub source: ThemeSource,
    pub day_start: String,
    pub night_start: String,
    pub day_brightness: f32,
    pub night_brightness: f32,
}
impl Default for DayNight {
    fn default() -> Self {
        Self {
            enabled: false,
            source: ThemeSource::Noctalia,
            day_start: "07:00".into(),
            night_start: "20:00".into(),
            day_brightness: 1.0,
            night_brightness: 0.65,
        }
    }
}
pub fn parse_clock(value: &str) -> Result<u32> {
    let Some((h, m)) = value.split_once(':') else {
        bail!("clock must use HH:MM: {value}");
    };
    let h: u32 = h.parse().context("invalid clock hour")?;
    let m: u32 = m.parse().context("invalid clock minute")?;
    if h > 23 || m > 59 {
        bail!("invalid clock: {value}");
    }
    Ok(h * 60 + m)
}
