use anyhow::Result;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

pub struct Wake(OwnedFd);
impl Wake {
    pub fn new() -> Result<Self> {
        let fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(Self(unsafe { OwnedFd::from_raw_fd(fd) }))
    }
    pub fn fd(&self) -> i32 {
        self.0.as_raw_fd()
    }
    pub fn notify(&self) {
        let n = 1u64;
        unsafe {
            libc::write(self.fd(), (&n as *const u64).cast(), 8);
        }
    }
    pub fn drain(&self) {
        let mut n = 0u64;
        unsafe {
            libc::read(self.fd(), (&mut n as *mut u64).cast(), 8);
        }
    }
}
