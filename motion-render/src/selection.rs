//! Selection policy independent of graphics, driven by monotonic deadlines.
use crate::config::{parse_clock, Config, Period, ThemeSource};
use std::time::{Duration, Instant};

pub fn schedule(config: &Config, minute: u32) -> Period {
    let day = parse_clock(&config.day_night.day_start).expect("validated time");
    let night = parse_clock(&config.day_night.night_start).expect("validated time");
    let is_day = if day < night {
        minute >= day && minute < night
    } else {
        minute >= day || minute < night
    };
    if is_day {
        Period::Day
    } else {
        Period::Night
    }
}
pub fn local_minute() -> u32 {
    unsafe {
        let mut time = std::mem::zeroed();
        let mut now = 0;
        libc::time(&mut now);
        libc::localtime_r(&now, &mut time);
        (time.tm_hour * 60 + time.tm_min) as u32
    }
}
pub fn resolve(
    config: &Config,
    manual: Option<Period>,
    theme: Option<Period>,
    minute: u32,
) -> Period {
    if let Some(period) = manual {
        return period;
    }
    if !config.day_night.enabled {
        return Period::Any;
    }
    if config.day_night.source == ThemeSource::Noctalia {
        if let Some(theme) = theme {
            return theme;
        }
    }
    schedule(config, minute)
}
pub fn adjacent(config: &Config, period: Period, backwards: bool) -> Option<usize> {
    let pool = config.eligible(period);
    if pool.is_empty() {
        return None;
    }
    let index = config.index();
    match pool.iter().position(|i| *i == index) {
        Some(pos) => Some(pool[(pos + if backwards { pool.len() - 1 } else { 1 }) % pool.len()]),
        None => Some(pool[if backwards { pool.len() - 1 } else { 0 }]),
    }
}
pub struct RotationClock {
    pub due: Instant,
}
impl RotationClock {
    pub fn new(now: Instant, seconds: u64) -> Self {
        Self {
            due: now + Duration::from_secs(seconds),
        }
    }
    pub fn reset(&mut self, now: Instant, seconds: u64) {
        self.due = now + Duration::from_secs(seconds);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Config {
        toml::from_str(
            r#"
active="day"
[day_night]
enabled=true
source="schedule"
[rotation]
enabled=true
interval_seconds=600
[[wallpapers]]
name="day"
type="shader"
source="a.frag"
period="day"
[[wallpapers]]
name="night"
type="video"
source="b.mp4"
period="night"
[[wallpapers]]
name="both"
type="image"
source="c.png"
[[wallpapers]]
name="disabled"
type="shader"
source="missing.frag"
enabled=false
"#,
        )
        .unwrap()
    }
    #[test]
    fn filters_and_wraps() {
        let mut c = config();
        assert_eq!(c.eligible(Period::Day), vec![0, 2]);
        assert_eq!(adjacent(&c, Period::Day, false), Some(2));
        assert_eq!(adjacent(&c, Period::Day, true), Some(2));
        c.active = "both".into();
        assert_eq!(adjacent(&c, Period::Day, false), Some(0));
        assert_eq!(adjacent(&c, Period::Night, false), Some(1));
    }
    #[test]
    fn schedule_boundaries_and_midnight() {
        let mut c = config();
        assert_eq!(schedule(&c, 419), Period::Night);
        assert_eq!(schedule(&c, 420), Period::Day);
        assert_eq!(schedule(&c, 1200), Period::Night);
        c.day_night.day_start = "20:00".into();
        c.day_night.night_start = "07:00".into();
        assert_eq!(schedule(&c, 0), Period::Day);
        assert_eq!(schedule(&c, 600), Period::Night);
    }
    #[test]
    fn theme_and_override() {
        let mut c = config();
        c.day_night.source = ThemeSource::Noctalia;
        assert_eq!(resolve(&c, None, Some(Period::Night), 600), Period::Night);
        assert_eq!(
            resolve(&c, Some(Period::Day), Some(Period::Night), 0),
            Period::Day
        );
        assert_eq!(resolve(&c, None, None, 600), Period::Day);
        c.day_night.enabled = false;
        assert_eq!(resolve(&c, None, None, 0), Period::Any);
    }
    #[test]
    fn ten_minute_deadline_resets_without_catchup() {
        let now = Instant::now();
        let mut clock = RotationClock::new(now, 600);
        assert_eq!(clock.due.duration_since(now), Duration::from_secs(600));
        clock.reset(now + Duration::from_secs(1000), 600);
        assert_eq!(clock.due.duration_since(now), Duration::from_secs(1600));
    }
    #[test]
    fn rejects_empty_modes_and_bad_times() {
        let mut c = config();
        c.day_night.day_start = "25:00".into();
        assert!(c.validate().is_err());
        c.day_night.day_start = "07:00".into();
        c.wallpapers[1].enabled = false;
        c.wallpapers[2].enabled = false;
        assert!(c.validate().is_err());
    }
}
