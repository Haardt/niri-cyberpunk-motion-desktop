use crate::color::Color;
use getopts::Options;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::env;
use std::fs::read_to_string;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum OutputMode {
    #[default]
    All,
    Active,
}

fn default_scale() -> u32 {
    1
}
fn default_background() -> Color {
    Color::new(0.0, 0.0, 0.0, 0.9)
}
fn default_cmd() -> String {
    "".to_string()
}
fn default_headline() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}
fn default_prompt() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}
fn default_prompt_err() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}
fn default_border() -> Color {
    Color::new(1.0, 1.0, 1.0, 1.0)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub output_mode: OutputMode,
    #[serde(default = "default_scale")]
    pub scale: u32,
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f64,
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default)]
    pub glitch: Glitch,
    #[serde(default = "default_background")]
    pub background: Color,
    #[serde(default = "default_headline")]
    pub headline: Color,
    #[serde(default = "default_prompt")]
    pub prompt: Color,
    #[serde(default = "default_prompt_err")]
    pub prompt_err: Color,
    #[serde(default = "default_border")]
    pub border: Color,
    #[serde(default = "default_cmd")]
    pub command: String,
    #[serde(default)]
    pub default_user: String,
    #[serde(default = "default_desktop")]
    pub desktop: String,
    #[serde(default = "default_session_type")]
    pub session_type: String,
}

fn default_desktop() -> String {
    "niri".into()
}
fn default_session_type() -> String {
    "wayland".into()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            output_mode: Default::default(),
            scale: 1,
            ui_scale: default_ui_scale(),
            title: default_title(),
            glitch: Glitch::default(),
            background: Color::new(0.0, 0.0, 0.0, 0.9),
            headline: Color::new(1.0, 1.0, 1.0, 1.0),
            prompt: Color::new(1.0, 1.0, 1.0, 1.0),
            prompt_err: Color::new(1.0, 1.0, 1.0, 1.0),
            border: Color::new(1.0, 1.0, 1.0, 1.0),
            command: "".to_string(),
            default_user: String::new(),
            desktop: default_desktop(),
            session_type: default_session_type(),
        }
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options]", program);
    print!("{}", opts.usage(&brief));
}

pub fn read_config() -> Config {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optflag("", "frontend-worker", "internal UI worker");
    opts.optflag("h", "help", "print this help menu");
    opts.optopt("c", "config", "config file to use", "CONFIG_FILE");
    opts.optopt("e", "command", "command to run", "COMMAND");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("{}", f.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(&program, opts);
        std::process::exit(0);
    }

    let mut config: Config = match read_to_string(
        matches
            .opt_str("config")
            .unwrap_or_else(|| "/etc/greetd/wlgreet.toml".to_string()),
    ) {
        Ok(s) => match toml::from_str(&s) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Unable to parse configuration file: {:?}", e);
                eprintln!("Please fix the configuration file and try again.");
                std::process::exit(1);
            }
        },
        Err(_) => Default::default(),
    };

    config.command = matches.opt_get_default("command", config.command).unwrap();

    config.validate().expect("invalid login configuration");
    config
}

fn default_title() -> String {
    "IDENTITY ACCESS".into()
}

fn default_ui_scale() -> f64 {
    1.35
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Glitch {
    pub enabled: bool,
    pub start_delay_seconds: u32,
    pub interval_seconds: u32,
    pub duration_ms: u32,
    pub strength: f32,
}
impl Default for Glitch {
    fn default() -> Self {
        Self {
            enabled: true,
            start_delay_seconds: 3,
            interval_seconds: 10,
            duration_ms: 350,
            strength: 0.32,
        }
    }
}
impl Config {
    pub fn buffer_size(&self, design: (u32, u32)) -> (i32, i32) {
        let factor = self.ui_scale * self.scale as f64;
        (
            (design.0 as f64 * factor).round() as i32,
            (design.1 as f64 * factor).round() as i32,
        )
    }
    pub fn design_position(&self, (x, y): (u32, u32)) -> (u32, u32) {
        let factor = self.ui_scale * self.scale as f64;
        ((x as f64 / factor) as u32, (y as f64 / factor) as u32)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.default_user.len() > 256
            || self
                .default_user
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(
                "defaultUser must be a username without whitespace (at most 256 bytes)".into(),
            );
        }
        if self.desktop.is_empty()
            || self.desktop.len() > 256
            || self
                .desktop
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || c == '=')
        {
            return Err(
                "desktop must be a nonempty desktop identifier without whitespace or '='".into(),
            );
        }
        if !matches!(self.session_type.as_str(), "wayland" | "x11") {
            return Err("sessionType must be wayland or x11".into());
        }
        if self.title.chars().count() > 64 || self.title.chars().any(char::is_control) {
            return Err("title must be a single line with at most 64 characters".into());
        }
        if !(1..=4).contains(&self.scale) {
            return Err("scale must be between 1 and 4 (buffer density)".into());
        }
        if !self.ui_scale.is_finite() || !(0.75..=2.5).contains(&self.ui_scale) {
            return Err("uiScale must be between 0.75 and 2.5".into());
        }
        if self.glitch.start_delay_seconds > 3600
            || !(5..=3600).contains(&self.glitch.interval_seconds)
            || !(80..=500).contains(&self.glitch.duration_ms)
            || !self.glitch.strength.is_finite()
            || !(0.0..=0.35).contains(&self.glitch.strength)
        {
            return Err(
                "glitch: startDelaySeconds 0..3600, intervalSeconds 5..3600, durationMs 80..500, strength 0..0.35".into(),
            );
        }
        Ok(())
    }
    pub fn session_env(&self) -> Vec<String> {
        vec![
            format!("XDG_SESSION_TYPE={}", self.session_type),
            format!("XDG_CURRENT_DESKTOP={}", self.desktop),
            format!("XDG_SESSION_DESKTOP={}", self.desktop),
            "XDG_SESSION_CLASS=user".into(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_and_desktop_settings_validate_and_export() {
        let config: Config = toml::from_str(
            "defaultUser = 'held'\ncommand = 'startplasma-wayland'\ndesktop = 'KDE'\nsessionType = 'wayland'",
        ).unwrap();
        config.validate().unwrap();
        assert_eq!(config.default_user, "held");
        assert!(config
            .session_env()
            .contains(&"XDG_CURRENT_DESKTOP=KDE".into()));
        for invalid in [
            "defaultUser = 'bad user'",
            "desktop = 'bad=env'",
            "sessionType = 'invalid'",
        ] {
            let config: Config = toml::from_str(invalid).unwrap();
            assert!(config.validate().is_err());
        }
    }
    #[test]
    fn ui_scale_keeps_clicks_aligned_at_multiple_densities() {
        let mut config = Config {
            ui_scale: 1.5,
            ..Config::default()
        };
        assert_eq!(config.buffer_size((800, 580)), (1200, 870));
        assert_eq!(config.design_position((150, 630)), (100, 420));
        config.scale = 2;
        assert_eq!(config.buffer_size((800, 580)), (2400, 1740));
        assert_eq!(config.design_position((300, 1260)), (100, 420));
        for scale in [0.0, f64::NAN, 3.0] {
            config.ui_scale = scale;
            assert!(config.validate().is_err());
        }
        config.ui_scale = 1.35;
        config.glitch.duration_ms = 3000;
        assert!(config.validate().is_err());
    }
}

#[cfg(test)]
mod title_tests {
    use super::*;
    #[test]
    fn title_defaults_and_unicode_override() {
        let old: Config = toml::from_str(r#"command = "niri-session""#).unwrap();
        assert_eq!(old.title, "IDENTITY ACCESS");
        let custom: Config = toml::from_str(r#"title = "Welcome, explorer 🙂""#).unwrap();
        assert_eq!(custom.title, "Welcome, explorer 🙂");
        custom.validate().unwrap();
        let invalid = Config {
            title: "bad\ntitle".into(),
            ..Config::default()
        };
        assert!(invalid.validate().is_err());
    }
}
