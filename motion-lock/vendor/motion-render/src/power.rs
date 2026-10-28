pub fn on_battery() -> bool {
    let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") else {
        return false;
    };
    entries.flatten().any(|e| {
        std::fs::read_to_string(e.path().join("type")).is_ok_and(|s| s.trim() == "Battery")
            && std::fs::read_to_string(e.path().join("status"))
                .is_ok_and(|s| s.trim() == "Discharging")
    })
}
