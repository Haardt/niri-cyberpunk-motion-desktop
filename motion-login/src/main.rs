mod app;
mod auth;
mod buffer;
mod color;
mod config;
mod draw;
mod panel;
mod widget;
mod widgets;

// The supervisor owns both children, including when authentication exits immediately.
static STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
extern "C" fn stop(_: i32) {
    STOP.store(true, std::sync::atomic::Ordering::Relaxed);
}
struct Children(Vec<std::process::Child>);
impl Drop for Children {
    fn drop(&mut self) {
        for child in &mut self.0 {
            if matches!(child.try_wait(), Ok(None)) {
                unsafe {
                    libc::kill(child.id() as i32, libc::SIGTERM);
                }
                for _ in 0..40 {
                    if !matches!(child.try_wait(), Ok(None)) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--background-worker") {
        env_logger::init();
        return motion_render::app::run(std::path::PathBuf::from(&args[2]));
    }
    if args.get(1).map(String::as_str) == Some("--frontend-worker") {
        return app::run(config::read_config());
    }
    if args.iter().any(|s| s == "--help" || s == "-h") {
        println!(
            "motion-login [--preview] [--config FILE] [--background FILE]\n\
                  motion-login --check [--config FILE] [--background FILE]\n\
                  Wayland greeter for greetd. Preview never connects to greetd; Escape closes it.\n\
                  Run inside a dedicated layer-shell compositor. SDDM is never changed."
        );
        return Ok(());
    }
    let option = |name: &str, fallback: &str| -> anyhow::Result<std::path::PathBuf> {
        match args.iter().position(|a| a == name) {
            Some(i) => Ok(args
                .get(i + 1)
                .ok_or_else(|| anyhow::anyhow!("missing value for {name}"))?
                .into()),
            None => Ok(fallback.into()),
        }
    };
    let config = option("--config", "/etc/motion-login/login.toml")?;
    let background = option("--background", "/etc/motion-login/background.toml")?;
    let login_config: config::Config = toml::from_str(&std::fs::read_to_string(&config)?)?;
    login_config.validate().map_err(anyhow::Error::msg)?;
    motion_render::config::Config::load(&background)?;
    if args.iter().any(|a| a == "--check") {
        println!("Login and shared renderer configuration valid");
        return Ok(());
    }
    let preview = args.iter().any(|a| a == "--preview");
    anyhow::ensure!(
        preview || std::env::var_os("GREETD_SOCK").is_some(),
        "GREETD_SOCK missing; use --preview outside greetd"
    );
    let exe = std::env::current_exe()?;
    let mut children = Children(Vec::new());
    // Separate socket and no desktop-specific niri/theme integration in a greeter.
    children.0.push(
        std::process::Command::new(&exe)
            .arg("--background-worker")
            .arg(background)
            .env(
                "MOTION_RENDER_SOCKET",
                format!("motion-login-{}.sock", std::process::id()),
            )
            .env_remove("NIRI_SOCKET")
            .spawn()?,
    );
    let mut ui = std::process::Command::new(exe);
    ui.arg("--frontend-worker").arg("--config").arg(config);
    if preview {
        ui.env("MOTION_LOGIN_PREVIEW", "1")
            .env_remove("GREETD_SOCK");
    }
    children.0.push(ui.spawn()?);
    unsafe {
        libc::signal(libc::SIGTERM, stop as *const () as usize);
        libc::signal(libc::SIGINT, stop as *const () as usize);
    }
    let mut background_exited = false;
    loop {
        if STOP.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        if let Some(status) = children.0[1].try_wait()? {
            anyhow::ensure!(status.success(), "login interface exited: {status}");
            return Ok(());
        }
        if !background_exited {
            if let Some(status) = children.0[0].try_wait()? {
                background_exited = true;
                eprintln!(
                    "Background stopped ({status}); login remains available with solid panel."
                );
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}
