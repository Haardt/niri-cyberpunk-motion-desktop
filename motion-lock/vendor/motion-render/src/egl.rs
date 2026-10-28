//! EGL objects are used only on the Wayland event/render thread.
use anyhow::{bail, ensure, Result};
use std::{
    ffi::{c_char, c_void, CString},
    ptr,
};
use wayland_client::{protocol::wl_surface::WlSurface, Connection, Proxy};
use wayland_egl::WlEglSurface;

type Handle = *mut c_void;
#[link(name = "EGL")]
unsafe extern "C" {
    fn eglGetDisplay(native: Handle) -> Handle;
    fn eglInitialize(display: Handle, major: *mut i32, minor: *mut i32) -> u32;
    fn eglBindAPI(api: u32) -> u32;
    fn eglChooseConfig(
        display: Handle,
        attrs: *const i32,
        configs: *mut Handle,
        size: i32,
        count: *mut i32,
    ) -> u32;
    fn eglCreateContext(
        display: Handle,
        config: Handle,
        share: Handle,
        attrs: *const i32,
    ) -> Handle;
    fn eglCreateWindowSurface(
        display: Handle,
        config: Handle,
        window: Handle,
        attrs: *const i32,
    ) -> Handle;
    fn eglMakeCurrent(display: Handle, draw: Handle, read: Handle, context: Handle) -> u32;
    fn eglSwapBuffers(display: Handle, surface: Handle) -> u32;
    fn eglSwapInterval(display: Handle, interval: i32) -> u32;
    fn eglDestroySurface(display: Handle, surface: Handle) -> u32;
    fn eglDestroyContext(display: Handle, context: Handle) -> u32;
    fn eglTerminate(display: Handle) -> u32;
    pub fn eglGetProcAddress(name: *const c_char) -> *const c_void;
}
pub struct Display {
    pub raw: Handle,
    config: Handle,
    context: Handle,
}
pub struct Surface {
    raw: Handle,
    native: WlEglSurface,
    display: Handle,
}
impl Display {
    pub fn new(conn: &Connection) -> Result<Self> {
        unsafe { Self::from_native(conn.backend().display_ptr().cast(), false) }
    }
    pub fn with_alpha(conn: &Connection) -> Result<Self> {
        unsafe { Self::from_native(conn.backend().display_ptr().cast(), true) }
    }
    /// Create an EGL context on the live native display.
    /// # Safety
    /// `native` must be a live wl_display and outlive this context and its surfaces.
    unsafe fn from_native(native: *mut c_void, alpha: bool) -> Result<Self> {
        unsafe {
            let raw = eglGetDisplay(native);
            ensure!(
                !raw.is_null() && eglInitialize(raw, ptr::null_mut(), ptr::null_mut()) != 0,
                "EGL initialization failed"
            );
            if eglBindAPI(0x30A2) == 0 {
                eglTerminate(raw);
                bail!("OpenGL unavailable");
            }
            let attrs = [
                0x3033,
                4,
                0x3040,
                8,
                0x3024,
                8,
                0x3023,
                8,
                0x3022,
                8,
                0x3021,
                if alpha { 8 } else { 0 },
                0x3038,
            ];
            let mut config = ptr::null_mut();
            let mut count = 0;
            if eglChooseConfig(raw, attrs.as_ptr(), &mut config, 1, &mut count) == 0 || count == 0 {
                eglTerminate(raw);
                bail!("No EGL OpenGL window configuration");
            }
            let context_attrs = [0x3098, 3, 0x30FB, 3, 0x30FD, 1, 0x3038];
            let context = eglCreateContext(raw, config, ptr::null_mut(), context_attrs.as_ptr());
            if context.is_null() {
                eglTerminate(raw);
                bail!("OpenGL 3.3 core context unavailable");
            }
            Ok(Self {
                raw,
                config,
                context,
            })
        }
    }
    pub fn surface(&self, wl: &WlSurface, width: i32, height: i32) -> Result<Surface> {
        unsafe { self.surface_native(wl.id().as_ptr().cast(), width, height) }
    }
    /// # Safety
    /// `wl` must be a live wl_surface from the same display, outliving the result.
    unsafe fn surface_native(&self, wl: *mut c_void, width: i32, height: i32) -> Result<Surface> {
        let native = unsafe { WlEglSurface::new_from_raw(wl.cast(), width, height)? };
        let raw = unsafe {
            eglCreateWindowSurface(self.raw, self.config, native.ptr().cast_mut(), ptr::null())
        };
        ensure!(!raw.is_null(), "eglCreateWindowSurface failed");
        Ok(Surface {
            raw,
            native,
            display: self.raw,
        })
    }
    pub fn current(&self, surface: &Surface) -> Result<()> {
        unsafe {
            ensure!(
                eglMakeCurrent(self.raw, surface.raw, surface.raw, self.context) != 0,
                "eglMakeCurrent failed"
            );
            // Compositor frame callbacks + our deadline provide pacing; avoid a second blocking vsync.
            eglSwapInterval(self.raw, 0);
        }
        Ok(())
    }
    pub fn gl(&self) -> glow::Context {
        unsafe {
            glow::Context::from_loader_function(|name| {
                eglGetProcAddress(CString::new(name).unwrap().as_ptr())
            })
        }
    }
}
impl Surface {
    pub fn resize(&self, w: i32, h: i32) {
        self.native.resize(w, h, 0, 0);
    }
    pub fn swap(&self) -> Result<()> {
        ensure!(
            unsafe { eglSwapBuffers(self.display, self.raw) } != 0,
            "eglSwapBuffers failed"
        );
        Ok(())
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            eglDestroySurface(self.display, self.raw);
        }
    }
}
impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            eglMakeCurrent(self.raw, ptr::null_mut(), ptr::null_mut(), ptr::null_mut());
            eglDestroyContext(self.raw, self.context);
            eglTerminate(self.raw);
        }
    }
}
