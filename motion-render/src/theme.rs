//! Query Noctalia off the render thread, with a strict process timeout.
use crate::{config::Period, wake::Wake};
use std::{
    io::Read,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};
fn query() -> Option<Period> {
    let mut child = Command::new("noctalia")
        .args(["msg", "theme-mode-get"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return None;
                }
                break;
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let mut output = String::new();
    child
        .stdout
        .take()?
        .take(1024)
        .read_to_string(&mut output)
        .ok()?;
    match output.trim() {
        "light" => Some(Period::Day),
        "dark" => Some(Period::Night),
        _ => None,
    }
}
pub fn watch(wake: Arc<Wake>, enabled: Arc<AtomicBool>) -> mpsc::Receiver<Option<Period>> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut previous = None;
        loop {
            let current = if enabled.load(Ordering::Relaxed) {
                query()
            } else {
                None
            };
            if current != previous {
                previous = current;
                if tx.send(current).is_err() {
                    break;
                }
                wake.notify();
            }
            thread::sleep(Duration::from_secs(30));
        }
    });
    rx
}
