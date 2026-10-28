//! Thin, dynamically loaded libmpv client/render ABI. All render API calls stay
//! on the EGL thread. Callbacks only set an atomic and signal an eventfd.
use crate::{config::Config, egl, wake::Wake};
use anyhow::{bail, ensure, Context, Result};
use libloading::Library;
use std::{
    ffi::{c_char, c_void, CStr, CString},
    path::Path,
    ptr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
type H = *mut c_void;
#[repr(C)]
struct Param {
    kind: i32,
    data: H,
}
#[repr(C)]
struct GlInit {
    get: unsafe extern "C" fn(H, *const c_char) -> H,
    ctx: H,
}
#[repr(C)]
struct Fbo {
    fbo: i32,
    w: i32,
    h: i32,
    format: i32,
}
#[repr(C)]
struct Event {
    id: i32,
    error: i32,
    userdata: u64,
    data: H,
}
#[repr(C)]
struct EndFile {
    reason: i32,
    error: i32,
    entry: i64,
    insert: i64,
    count: i32,
}
struct Api {
    create: unsafe extern "C" fn() -> H,
    initialize: unsafe extern "C" fn(H) -> i32,
    option: unsafe extern "C" fn(H, *const c_char, *const c_char) -> i32,
    property: unsafe extern "C" fn(H, *const c_char, *const c_char) -> i32,
    get: unsafe extern "C" fn(H, *const c_char) -> *mut c_char,
    command: unsafe extern "C" fn(H, *const *const c_char) -> i32,
    wait: unsafe extern "C" fn(H, f64) -> *mut Event,
    destroy: unsafe extern "C" fn(H),
    free: unsafe extern "C" fn(H),
    error: unsafe extern "C" fn(i32) -> *const c_char,
    render_create: unsafe extern "C" fn(*mut H, H, *mut Param) -> i32,
    render_free: unsafe extern "C" fn(H),
    update_cb: unsafe extern "C" fn(H, Option<unsafe extern "C" fn(H)>, H),
    wake_cb: unsafe extern "C" fn(H, Option<unsafe extern "C" fn(H)>, H),
    update: unsafe extern "C" fn(H) -> u64,
    render: unsafe extern "C" fn(H, *mut Param) -> i32,
    swap: unsafe extern "C" fn(H),
    _lib: Library,
}
impl Api {
    fn load() -> Result<Self> {
        unsafe {
            let lib = Library::new("libmpv.so.2")
                .context("video needs libmpv.so.2 (on Arch/CachyOS: sudo pacman -S mpv)")?;
            macro_rules! sym {
                ($name:literal) => {
                    *lib.get(concat!($name, "\0").as_bytes())?
                };
            }
            Ok(Self {
                create: sym!("mpv_create"),
                initialize: sym!("mpv_initialize"),
                option: sym!("mpv_set_option_string"),
                property: sym!("mpv_set_property_string"),
                get: sym!("mpv_get_property_string"),
                command: sym!("mpv_command"),
                wait: sym!("mpv_wait_event"),
                destroy: sym!("mpv_terminate_destroy"),
                free: sym!("mpv_free"),
                error: sym!("mpv_error_string"),
                render_create: sym!("mpv_render_context_create"),
                render_free: sym!("mpv_render_context_free"),
                update_cb: sym!("mpv_render_context_set_update_callback"),
                wake_cb: sym!("mpv_set_wakeup_callback"),
                update: sym!("mpv_render_context_update"),
                render: sym!("mpv_render_context_render"),
                swap: sym!("mpv_render_context_report_swap"),
                _lib: lib,
            })
        }
    }
    fn check(&self, code: i32) -> Result<()> {
        if code < 0 {
            bail!(
                "libmpv: {}",
                unsafe { CStr::from_ptr((self.error)(code)) }.to_string_lossy()
            );
        }
        Ok(())
    }
}
struct Callback {
    wake: Arc<Wake>,
    dirty: AtomicBool,
}
unsafe extern "C" fn notify(ctx: H) {
    let cb = &*(ctx as *const Callback);
    cb.dirty.store(true, Ordering::Release);
    cb.wake.notify();
}
unsafe extern "C" fn get_proc(_: H, name: *const c_char) -> H {
    egl::eglGetProcAddress(name) as H
}
pub struct Video {
    api: Api,
    handle: H,
    render: H,
    callback: Box<Callback>,
    pub decoder: String,
    require_hw: bool,
    pub failed: bool,
    paused: bool,
    pending: bool,
}
impl Video {
    pub fn new(path: &Path, config: &Config, display: H, wake: Arc<Wake>) -> Result<Self> {
        let api = Api::load()?;
        let handle = unsafe { (api.create)() };
        ensure!(!handle.is_null(), "mpv_create failed");
        let mut video = Self {
            api,
            handle,
            render: ptr::null_mut(),
            callback: Box::new(Callback {
                wake,
                dirty: AtomicBool::new(true),
            }),
            decoder: "loading".into(),
            require_hw: config.require_hardware_video,
            failed: false,
            paused: false,
            pending: true,
        };
        for (k, v) in [
            ("config", "no"),
            ("vo", "libmpv"),
            ("hwdec", config.hwdec.as_str()),
            ("audio", "no"),
            ("sub", "no"),
            ("osc", "no"),
            ("terminal", "no"),
            ("input-default-bindings", "no"),
            ("input-vo-keyboard", "no"),
            ("loop-file", "inf"),
            ("keep-open", "yes"),
            ("panscan", "1.0"),
            ("video-sync", "desync"),
            ("vd-lavc-threads", "2"),
        ] {
            video.api.check(unsafe {
                (video.api.option)(handle, CString::new(k)?.as_ptr(), CString::new(v)?.as_ptr())
            })?;
        }
        video.api.check(unsafe { (video.api.initialize)(handle) })?;
        let cb = (&mut *video.callback as *mut Callback).cast();
        unsafe {
            (video.api.wake_cb)(handle, Some(notify), cb);
        }
        let mut init = GlInit {
            get: get_proc,
            ctx: ptr::null_mut(),
        };
        let mut params = [
            Param {
                kind: 1,
                data: c"opengl".as_ptr() as H,
            },
            Param {
                kind: 2,
                data: (&mut init as *mut GlInit).cast(),
            },
            Param {
                kind: 9,
                data: display,
            },
            Param {
                kind: 0,
                data: ptr::null_mut(),
            },
        ];
        video.api.check(unsafe {
            (video.api.render_create)(&mut video.render, handle, params.as_mut_ptr())
        })?;
        unsafe {
            (video.api.update_cb)(video.render, Some(notify), cb);
        }
        let path = CString::new(path.as_os_str().as_encoded_bytes())?;
        let args = [c"loadfile".as_ptr(), path.as_ptr(), ptr::null()];
        video
            .api
            .check(unsafe { (video.api.command)(handle, args.as_ptr()) })?;
        Ok(video)
    }
    fn string(&self, name: &CStr) -> String {
        unsafe {
            let p = (self.api.get)(self.handle, name.as_ptr());
            if p.is_null() {
                return String::new();
            }
            let s = CStr::from_ptr(p).to_string_lossy().into_owned();
            (self.api.free)(p.cast());
            s
        }
    }
    pub fn update(&mut self) -> Result<()> {
        if !self.callback.dirty.swap(false, Ordering::AcqRel) {
            return Ok(());
        }
        loop {
            unsafe {
                let ev = &*(self.api.wait)(self.handle, 0.0);
                if ev.id == 0 {
                    break;
                }
                if ev.id == 17 {
                    // MPV_EVENT_VIDEO_RECONFIG: decoder is initialized by now
                    self.decoder = self.string(c"hwdec-current");
                    if self.decoder.is_empty() || self.decoder == "no" {
                        self.decoder = "software".into();
                    }
                    log::info!("video decoder: {}", self.decoder);
                    if self.require_hw && self.decoder == "software" {
                        self.failed = true;
                        self.pause(true)?;
                        bail!("hardware decoding required but unavailable for this video");
                    }
                }
                if ev.id == 7 && !ev.data.is_null() {
                    let end = &*(ev.data as *const EndFile);
                    if end.reason == 4 {
                        self.failed = true;
                        self.pause(true)?;
                        self.api.check(end.error)?;
                        bail!("video playback failed");
                    }
                }
            }
        }
        if unsafe { (self.api.update)(self.render) } & 1 != 0 {
            self.pending = true;
        }
        Ok(())
    }
    pub fn pending(&self) -> bool {
        self.pending
    }
    pub fn pause(&mut self, value: bool) -> Result<()> {
        if value != self.paused {
            self.api.check(unsafe {
                (self.api.property)(
                    self.handle,
                    c"pause".as_ptr(),
                    if value { c"yes" } else { c"no" }.as_ptr(),
                )
            })?;
            self.paused = value;
        }
        Ok(())
    }
    pub fn draw(&mut self, w: i32, h: i32) -> Result<()> {
        let mut fbo = Fbo {
            fbo: 0,
            w,
            h,
            format: 0,
        };
        let mut flip = 1i32;
        let mut block = 0i32;
        let mut params = [
            Param {
                kind: 3,
                data: (&mut fbo as *mut Fbo).cast(),
            },
            Param {
                kind: 4,
                data: (&mut flip as *mut i32).cast(),
            },
            Param {
                kind: 12,
                data: (&mut block as *mut i32).cast(),
            },
            Param {
                kind: 0,
                data: ptr::null_mut(),
            },
        ];
        self.api
            .check(unsafe { (self.api.render)(self.render, params.as_mut_ptr()) })?;
        self.pending = false;
        Ok(())
    }
    pub fn swapped(&self) {
        unsafe {
            (self.api.swap)(self.render);
        }
    }
}
impl Drop for Video {
    fn drop(&mut self) {
        unsafe {
            (self.api.wake_cb)(self.handle, None, ptr::null_mut());
            if !self.render.is_null() {
                (self.api.update_cb)(self.render, None, ptr::null_mut());
                (self.api.render_free)(self.render);
            }
            (self.api.destroy)(self.handle);
        }
    }
}
