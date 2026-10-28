//! Exercise the isolated helper with the host's deny-all PAM service.
use std::process::{Command, Stdio};

#[test]
fn biometric_helper_denial_and_missing_service() {
    let binary = env!("CARGO_BIN_EXE_motion-lock");
    let missing = Command::new(binary)
        .args([
            "--fingerprint-worker",
            "--pam-service",
            "motion-lock-missing-test",
        ])
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(!String::from_utf8_lossy(&missing.stdout).contains("\"success\":true"));
    if !std::path::Path::new("/etc/pam.d/other").exists() {
        return;
    }
    let mut child = Command::new(binary)
        .args(["--fingerprint-worker", "--pam-service", "other"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("biometric denial timed out");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let events: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(events
        .iter()
        .any(|e| e["event"] == "complete" && e["success"] == false));
    assert!(!events.iter().any(|e| e["success"] == true));
}
