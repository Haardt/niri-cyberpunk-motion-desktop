use anyhow::{bail, Context, Result};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{OpenOptionsExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    path::PathBuf,
    time::Duration,
};
pub fn socket_path() -> Result<PathBuf> {
    Ok(
        PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").context("XDG_RUNTIME_DIR is required")?)
            .join(
                std::env::var("MOTION_RENDER_SOCKET")
                    .unwrap_or_else(|_| "niri-motion-background.sock".into()),
            ),
    )
}
pub struct Server {
    pub listener: UnixListener,
    path: PathBuf,
    _lock: File,
}
impl Server {
    pub fn bind() -> Result<Self> {
        let path = socket_path()?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(path.with_extension("lock"))?;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            bail!("another niri-motion-background instance is running");
        }
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        let listener = UnixListener::bind(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            path,
            _lock: lock,
        })
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
pub fn send(command: &str) -> Result<()> {
    let mut s = UnixStream::connect(socket_path()?).context("daemon is not running")?;
    s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.write_all(command.as_bytes())?;
    s.write_all(b"\n")?;
    let mut response = String::new();
    s.take(65536).read_to_string(&mut response)?;
    println!("{response}");
    if serde_json::from_str::<serde_json::Value>(&response)?
        .get("error")
        .is_some()
    {
        bail!("daemon rejected command");
    }
    Ok(())
}
