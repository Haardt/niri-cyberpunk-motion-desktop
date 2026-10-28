//! Keep presentation deadlines on a steady cadence instead of accumulating
//! scheduler/render lateness on every frame. Missed slots are skipped, not burst.
use std::time::{Duration, Instant};

pub fn next_deadline(previous: Instant, interval: Duration, now: Instant) -> Instant {
    debug_assert!(!interval.is_zero());
    if previous > now {
        return previous;
    }
    let slots = now.duration_since(previous).as_nanos() / interval.as_nanos() + 1;
    match u32::try_from(slots) {
        Ok(slots) => previous + interval * slots,
        Err(_) => now + interval,
    }
}

pub fn animation_step(
    elapsed: Duration,
    was_running: bool,
    running: bool,
    interval: Duration,
) -> Duration {
    if was_running && running {
        elapsed.min(interval.max(Duration::from_millis(250)))
    } else {
        Duration::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversleep_does_not_accumulate_drift() {
        let start = Instant::now();
        let interval = Duration::from_millis(50);
        let mut due = start;
        for i in 0..100 {
            let actual = due + Duration::from_millis(2);
            due = next_deadline(due, interval, actual);
            assert_eq!(due, start + interval * (i + 1));
        }
    }
    #[test]
    fn missed_frames_skip_without_burst() {
        let start = Instant::now();
        let interval = Duration::from_millis(50);
        assert_eq!(
            next_deadline(start, interval, start + Duration::from_millis(217)),
            start + Duration::from_millis(250)
        );
        assert_eq!(
            next_deadline(start, interval, start + interval),
            start + interval * 2
        );
    }
    #[test]
    fn animation_uses_elapsed_time_and_freezes_on_pause() {
        let interval = Duration::from_millis(83);
        let elapsed = Duration::from_millis(90);
        assert_eq!(animation_step(elapsed, true, true, interval), elapsed);
        assert_eq!(
            animation_step(elapsed, true, false, interval),
            Duration::ZERO
        );
        assert_eq!(
            animation_step(elapsed, false, true, interval),
            Duration::ZERO
        );
        assert_eq!(
            animation_step(Duration::from_secs(30), true, true, interval),
            Duration::from_millis(250)
        );
        assert_eq!(
            animation_step(Duration::from_secs(1), true, true, Duration::from_secs(1)),
            Duration::from_secs(1)
        );
    }
}
