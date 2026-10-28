//! motion-lock: preview and real session locking are separate, explicit modes.
mod auth;
#[allow(dead_code)]
mod buffer;
#[allow(dead_code)]
mod color;
mod config;
#[allow(dead_code)]
mod draw;
mod interface;
mod render;

use anyhow::{bail, Context, Result};
use clap::{ArgGroup, Parser};
use config::Config;
use motion_render::egl;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData},
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{EventLoop, LoopHandle},
        calloop_wayland_source::WaylandSource,
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    session_lock::{
        SessionLock, SessionLockHandler, SessionLockState, SessionLockSurface,
        SessionLockSurfaceConfigure,
    },
    shell::{
        xdg::{
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
            XdgShell,
        },
        WaylandSurface,
    },
};
use std::{
    path::PathBuf,
    sync::mpsc::TryRecvError,
    time::{Duration, Instant},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{
        wl_keyboard::WlKeyboard,
        wl_output::{Transform, WlOutput},
        wl_pointer::WlPointer,
        wl_seat::WlSeat,
        wl_surface::WlSurface,
    },
    Connection, QueueHandle,
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Parser)]
#[command(version,about="Shader-backed Wayland session locker. Preview never locks or authenticates.",group(ArgGroup::new("mode").required(true).args(["preview","lock","check","fingerprint_worker"])))]
struct Args {
    #[arg(long)]
    preview: bool,
    #[arg(long, hide = true)]
    fingerprint_worker: bool,
    #[arg(long)]
    lock: bool,
    #[arg(long)]
    check: bool,
    #[arg(short, long)]
    config: Option<PathBuf>,
    #[arg(long)]
    shader: Option<PathBuf>,
    #[arg(long)]
    pam_service: Option<String>,
    /// Close only the non-locking preview automatically.
    #[arg(long, requires = "preview", conflicts_with_all = ["lock", "check"])]
    preview_seconds: Option<u64>,
}
enum Role {
    Lock(SessionLockSurface),
    Preview(Window),
}
struct View {
    // EGL must be destroyed before its wl_surface role.
    egl: Option<egl::Surface>,
    role: Role,
    output: Option<WlOutput>,
    width: u32,
    height: u32,
    scale: i32,
    ready: bool,
    frame_pending: bool,
}
impl View {
    fn surface(&self) -> &WlSurface {
        match &self.role {
            Role::Lock(s) => s.wl_surface(),
            Role::Preview(w) => w.wl_surface(),
        }
    }
}
struct App {
    // Views are dropped before the renderer's EGL display.
    views: Vec<View>,
    renderer: render::Renderer,
    conn: Connection,
    compositor: CompositorState,
    outputs: OutputState,
    registry: RegistryState,
    seats: SeatState,
    lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    _xdg: Option<XdgShell>,
    keyboards: Vec<(WlSeat, WlKeyboard)>,
    pointers: Vec<(WlSeat, WlPointer)>,
    loop_handle: LoopHandle<'static, Self>,
    modifiers: Modifiers,
    config: Config,
    preview: bool,
    locked: bool,
    exit: bool,
    auth: Option<auth::Auth>,
    fingerprint: Option<auth::Fingerprint>,
    fingerprint_retry: Instant,
    fingerprint_status: String,
    answer: Zeroizing<String>,
    question: String,
    secret: bool,
    status: String,
    pending: bool,
    waiting: bool,
    last_attempt: Option<Instant>,
    start: Instant,
    last_draw: Instant,
    last_second: i64,
    battery: bool,
    power_checked: Instant,
}
fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    if args.fingerprint_worker {
        return auth::fingerprint_worker(
            args.pam_service
                .as_deref()
                .unwrap_or("motion-lock-fingerprint"),
        );
    }
    let mut cfg = Config::load(args.config.as_deref())?;
    if args.shader.is_some() {
        cfg.shader = args.shader.clone();
    }
    if let Some(s) = args.pam_service {
        cfg.pam_service = s;
    }
    let shader = cfg.shader_path()?;
    if args.check {
        println!(
            "Shader: {}\nUI scale: {}\nFPS: {} AC / {} battery",
            shader.display(),
            cfg.ui_scale,
            cfg.fps,
            cfg.battery_fps
        );
        println!(
            "PAM service: {} ({})",
            cfg.pam_service,
            if PathBuf::from("/etc/pam.d").join(&cfg.pam_service).is_file() {
                "installed"
            } else {
                "not installed; real lock will refuse to start"
            }
        );
        println!(
            "Fingerprint: {} (PAM: {})",
            cfg.fingerprint, cfg.fingerprint_pam_service
        );
        return Ok(());
    }
    // Do not create the PAM worker or even open a PAM context in preview mode.
    let auth = if args.lock {
        if unsafe { libc::getuid() } == 0 {
            bail!("run motion-lock as the desktop user, not root");
        }
        unsafe {
            let limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            libc::setrlimit(libc::RLIMIT_CORE, &limit);
            libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
        }
        Some(auth::Auth::new(&cfg.pam_service)?)
    } else {
        None
    };
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init(&conn)?;
    let qh = queue.handle();
    let mut event_loop: EventLoop<App> = EventLoop::try_new()?;
    let xdg = if args.preview {
        Some(XdgShell::bind(&globals, &qh)?)
    } else {
        None
    };
    let mut app = App {
        views: vec![],
        renderer: render::Renderer::new(&conn, &shader)?,
        conn: conn.clone(),
        compositor: CompositorState::bind(&globals, &qh)?,
        outputs: OutputState::new(&globals, &qh),
        registry: RegistryState::new(&globals),
        seats: SeatState::new(&globals, &qh),
        lock_state: SessionLockState::new(&globals, &qh),
        session_lock: None,
        _xdg: xdg,
        keyboards: vec![],
        pointers: vec![],
        loop_handle: event_loop.handle(),
        modifiers: Modifiers::default(),
        config: cfg,
        preview: args.preview,
        locked: false,
        exit: false,
        auth,
        fingerprint: None,
        fingerprint_retry: Instant::now(),
        fingerprint_status: String::new(),
        answer: Zeroizing::new(String::new()),
        question: format!("Password for {}", whoami::username()),
        secret: true,
        status: String::new(),
        pending: false,
        waiting: true,
        last_attempt: None,
        start: Instant::now(),
        last_draw: Instant::now() - Duration::from_secs(1),
        last_second: 0,
        battery: motion_render::power::on_battery(),
        power_checked: Instant::now(),
    };
    // Receive output and seat capabilities before requesting any lock surfaces.
    queue.roundtrip(&mut app)?;
    let preflight_surface = app.compositor.create_surface(&qh);
    let preflight = app.renderer.prepare(&preflight_surface);
    preflight_surface.destroy();
    preflight.context("shader preflight failed before locking")?;
    if app.preview {
        let surface = app.compositor.create_surface(&qh);
        let window = app._xdg.as_ref().unwrap().create_window(
            surface,
            WindowDecorations::RequestServer,
            &qh,
        );
        window.set_title("Motion Lock — Preview (does not lock)");
        window.set_app_id("dev.motion.LockPreview");
        window.set_min_size(Some((400, 300)));
        window.commit();
        app.views.push(View {
            egl: None,
            role: Role::Preview(window),
            output: None,
            width: 1100,
            height: 800,
            scale: 1,
            ready: false,
            frame_pending: false,
        });
    } else {
        app.session_lock = Some(
            app.lock_state
                .lock(&qh)
                .context("ext-session-lock-v1 unavailable or already locked")?,
        );
    }
    if !app.preview {
        for output in app.outputs.outputs().collect::<Vec<_>>() {
            app.add_output(output, &qh);
        }
    }
    WaylandSource::new(conn.clone(), queue)
        .insert(event_loop.handle())
        .map_err(|e| anyhow::anyhow!("Wayland event source: {e}"))?;
    log::info!(
        "{}; shader {}",
        if app.preview {
            "non-locking preview"
        } else {
            "requesting session lock"
        },
        shader.display()
    );
    while !app.exit {
        let fps = if app.battery {
            app.config.battery_fps
        } else {
            app.config.fps
        };
        let wait = Duration::from_secs_f64(1. / fps as f64)
            .saturating_sub(app.last_draw.elapsed())
            .max(Duration::from_millis(2));
        event_loop.dispatch(wait, &mut app)?;
        app.poll_auth()?;
        app.poll_fingerprint()?;
        app.draw(&qh)?;
        conn.flush()?;
        if app.preview
            && args
                .preview_seconds
                .is_some_and(|s| app.start.elapsed() >= Duration::from_secs(s))
        {
            app.exit = true;
        }
    }
    Ok(())
}
impl App {
    fn add_output(&mut self, output: WlOutput, qh: &QueueHandle<Self>) {
        if self.preview
            || self
                .views
                .iter()
                .any(|v| v.output.as_ref() == Some(&output))
        {
            return;
        }
        let Some(lock) = &self.session_lock else {
            return;
        };
        let surface = self.compositor.create_surface(qh);
        let role = Role::Lock(lock.create_lock_surface(surface, &output, qh));
        let scale = self
            .outputs
            .info(&output)
            .map_or(1, |i| i.scale_factor)
            .max(1);
        self.views.push(View {
            egl: None,
            role,
            output: Some(output),
            width: 1,
            height: 1,
            scale,
            ready: false,
            frame_pending: false,
        });
    }
    fn submit(&mut self) {
        if self.preview {
            self.answer.zeroize();
            self.status = "PREVIEW // UNLOCK DISABLED".into();
            self.renderer.invalidate();
            return;
        }
        if !self.locked || !self.waiting || self.answer.is_empty() {
            return;
        }
        if !self.pending
            && self
                .last_attempt
                .is_some_and(|t| t.elapsed() < Duration::from_secs(1))
        {
            return;
        }
        let value = std::mem::replace(&mut self.answer, Zeroizing::new(String::new()));
        let result = self.auth.as_ref().unwrap().submit(value, self.pending);
        if result.is_ok() {
            self.pending = true;
            self.waiting = false;
            self.status = "AUTHENTICATING ...".into();
            self.last_attempt = Some(Instant::now());
        } else {
            self.status = "PAM UNAVAILABLE // SESSION REMAINS LOCKED".into();
        }
        self.renderer.invalidate();
    }
    fn key(&mut self, event: KeyEvent) {
        use smithay_client_toolkit::seat::keyboard::Keysym;
        if event.keysym == Keysym::Escape {
            self.answer.zeroize();
            if self.preview {
                self.exit = true;
            }
            self.renderer.invalidate();
            return;
        }
        if !self.waiting && !self.preview {
            return;
        }
        match event.keysym {
            Keysym::Return | Keysym::KP_Enter => self.submit(),
            Keysym::BackSpace => {
                self.answer.pop();
            }
            Keysym::u if self.modifiers.ctrl => self.answer.zeroize(),
            _ => {
                if !self.modifiers.ctrl && !self.modifiers.alt && !self.modifiers.logo {
                    if let Some(text) = event.utf8 {
                        for c in text.chars().filter(|c| !c.is_control()) {
                            if self.answer.len() + c.len_utf8() <= 1024 {
                                self.answer.push(c);
                            }
                        }
                    }
                }
            }
        }
        self.renderer.invalidate();
    }
    fn poll_auth(&mut self) -> Result<()> {
        let Some(auth) = &self.auth else {
            return Ok(());
        };
        let mut events = vec![];
        loop {
            match auth.events.try_recv() {
                Ok(e) => events.push(e),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.status = "PAM STOPPED // SESSION REMAINS LOCKED".into();
                    self.waiting = false;
                    self.renderer.invalidate();
                    break;
                }
            }
        }
        for event in events {
            match event {
                auth::Event::Prompt(prompt, secret) => {
                    self.question = prompt;
                    self.secret = secret;
                    self.waiting = true;
                    self.answer.zeroize();
                    self.status.clear();
                }
                auth::Event::Notice(msg) => self.status = msg,
                auth::Event::Result(success) => {
                    self.answer.zeroize();
                    let was_pending = self.pending;
                    self.pending = false;
                    self.waiting = true;
                    self.secret = true;
                    if success && was_pending && self.locked && !self.preview {
                        self.unlock_authenticated()?;
                    } else {
                        self.status = "ACCESS DENIED // TRY AGAIN".into();
                        self.question = format!("Password for {}", whoami::username());
                    }
                }
            }
            self.renderer.invalidate();
        }
        Ok(())
    }
    fn unlock_authenticated(&mut self) -> Result<()> {
        if !self.locked || self.preview || self.exit {
            return Ok(());
        }
        self.fingerprint.take();
        self.answer.zeroize();
        self.views.clear();
        self.session_lock
            .take()
            .context("authenticated without a lock")?
            .unlock();
        self.conn.flush()?;
        self.exit = true;
        log::info!("PAM credential and account checks passed; unlock sent");
        Ok(())
    }
    fn poll_fingerprint(&mut self) -> Result<()> {
        if self.preview || !self.locked || self.exit || !self.config.fingerprint {
            return Ok(());
        }
        if self.fingerprint.is_none() && Instant::now() >= self.fingerprint_retry {
            match auth::Fingerprint::start(&self.config.fingerprint_pam_service) {
                Ok(scan) => {
                    self.fingerprint = Some(scan);
                    self.fingerprint_status.clear();
                }
                Err(e) => {
                    log::warn!("Fingerprint unavailable: {e}");
                    self.fingerprint_retry = Instant::now() + Duration::from_secs(60);
                }
            }
        }
        let mut events = Vec::new();
        let mut ended = false;
        if let Some(scan) = &self.fingerprint {
            loop {
                match scan.events.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        ended = true;
                        break;
                    }
                }
            }
        }
        for event in events {
            match event {
                auth::FingerprintEvent::Notice { message } => self.fingerprint_status = message,
                auth::FingerprintEvent::Complete { success } => {
                    if success {
                        self.unlock_authenticated()?;
                        return Ok(());
                    }
                    ended = true;
                }
            }
            self.renderer.invalidate();
        }
        if ended {
            self.fingerprint.take();
            self.fingerprint_status.clear();
            self.fingerprint_retry = Instant::now() + Duration::from_secs(5);
            self.renderer.invalidate();
        }
        Ok(())
    }
    fn draw(&mut self, qh: &QueueHandle<Self>) -> Result<()> {
        if self.exit {
            return Ok(());
        }
        if self.power_checked.elapsed() > Duration::from_secs(5) {
            self.battery = motion_render::power::on_battery();
            self.power_checked = Instant::now();
        }
        let fps = if self.battery {
            self.config.battery_fps
        } else {
            self.config.fps
        };
        if self.last_draw.elapsed() < Duration::from_secs_f64(1. / fps as f64) {
            return Ok(());
        }
        let second = chrono::Local::now().timestamp();
        if second != self.last_second {
            self.last_second = second;
            self.renderer.invalidate();
        }
        // Only frame-ready outputs are rendered; a sleeping display schedules no GPU work.
        for view in &mut self.views {
            if !view.ready || view.frame_pending {
                continue;
            }
            let w = view.width as i32 * view.scale;
            let h = view.height as i32 * view.scale;
            if view.egl.is_none() {
                view.egl = Some(self.renderer.display.surface(view.surface(), w, h)?);
            }
            let egl = view.egl.as_ref().unwrap();
            egl.resize(w, h);
            self.renderer.display.current(egl)?;
            self.renderer.update(
                &self.question,
                &self.answer,
                self.secret,
                if self.status.is_empty() {
                    &self.fingerprint_status
                } else {
                    &self.status
                },
                self.preview,
                &self.config,
                self.pending && !self.waiting,
                self.fingerprint.is_some(),
            )?;
            // First draw initializes GL objects. Update the UI texture once that exists.
            self.renderer.draw(
                egl,
                w,
                h,
                view.scale,
                self.start.elapsed().as_secs_f32() * self.config.shader_speed,
                self.config.brightness,
                self.config.ui_scale,
            )?;
            self.renderer.update(
                &self.question,
                &self.answer,
                self.secret,
                if self.status.is_empty() {
                    &self.fingerprint_status
                } else {
                    &self.status
                },
                self.preview,
                &self.config,
                self.pending && !self.waiting,
                self.fingerprint.is_some(),
            )?;
            view.surface()
                .frame(qh, FrameCallbackData(view.surface().clone()));
            view.surface().commit();
            view.frame_pending = true;
        }
        self.last_draw = Instant::now();
        Ok(())
    }
}
impl SessionLockHandler for App {
    fn locked(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: SessionLock) {
        self.locked = true;
        let outputs: Vec<_> = self.outputs.outputs().collect();
        for o in outputs {
            self.add_output(o, qh);
        }
        log::info!("session lock confirmed");
    }
    fn finished(&mut self, _: &Connection, _: &QueueHandle<Self>, _: SessionLock) {
        self.answer.zeroize();
        self.exit = true;
        log::warn!("compositor finished or refused lock; no unlock sent");
    }
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        s: SessionLockSurface,
        c: SessionLockSurfaceConfigure,
        _: u32,
    ) {
        if let Some(v) = self
            .views
            .iter_mut()
            .find(|v| v.surface() == s.wl_surface())
        {
            let (w, h) = c.new_size;
            if w > 0 && h > 0 && w <= 16384 && h <= 16384 {
                v.width = w;
                v.height = h;
                v.ready = true;
                v.frame_pending = false;
                v.surface().set_buffer_scale(v.scale);
            }
        }
    }
}
impl WindowHandler for App {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        if self.preview {
            self.exit = true;
        }
    }
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        window: &Window,
        c: WindowConfigure,
        _: u32,
    ) {
        if let Some(v) = self
            .views
            .iter_mut()
            .find(|v| v.surface() == window.wl_surface())
        {
            v.width = c.new_size.0.map_or(v.width, |n| n.get());
            v.height = c.new_size.1.map_or(v.height, |n| n.get());
            v.ready = true;
            v.frame_pending = false;
        }
    }
}
impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        s: &WlSurface,
        f: i32,
    ) {
        if let Some(v) = self.views.iter_mut().find(|v| v.surface() == s) {
            v.scale = f.clamp(1, 4);
            v.surface().set_buffer_scale(v.scale);
            v.frame_pending = false;
            self.renderer.invalidate();
        }
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, s: &WlSurface, _: u32) {
        if let Some(v) = self.views.iter_mut().find(|v| v.surface() == s) {
            v.frame_pending = false;
        }
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlSurface,
        _: &WlOutput,
    ) {
    }
}
impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, o: WlOutput) {
        self.add_output(o, qh);
    }
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, o: WlOutput) {
        if let Some(i) = self.outputs.info(&o) {
            for v in &mut self.views {
                if v.output.as_ref() == Some(&o) {
                    v.scale = i.scale_factor.clamp(1, 4);
                    v.surface().set_buffer_scale(v.scale);
                    v.frame_pending = false;
                }
            }
        }
    }
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, o: WlOutput) {
        self.views.retain(|v| v.output.as_ref() != Some(&o));
    }
}
impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: WlSeat,
        c: Capability,
    ) {
        match c {
            Capability::Keyboard => {
                if let Ok(k) = self.seats.get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|s, _, e| s.key(e)),
                ) {
                    self.keyboards.push((seat, k));
                }
            }
            Capability::Pointer => {
                if let Ok(p) = self.seats.get_pointer(qh, &seat) {
                    self.pointers.push((seat, p));
                }
            }
            _ => {}
        }
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        seat: WlSeat,
        c: Capability,
    ) {
        if c == Capability::Keyboard {
            self.keyboards.retain(|(s, k)| {
                if s == &seat {
                    k.release();
                    false
                } else {
                    true
                }
            });
        }
        if c == Capability::Pointer {
            self.pointers.retain(|(s, p)| {
                if s == &seat {
                    p.release();
                    false
                } else {
                    true
                }
            });
        }
    }
    fn remove_seat(&mut self, c: &Connection, qh: &QueueHandle<Self>, s: WlSeat) {
        self.remove_capability(c, qh, s.clone(), Capability::Keyboard);
        self.remove_capability(c, qh, s, Capability::Pointer);
    }
}
impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: &WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: &WlSurface,
        _: u32,
    ) {
        self.answer.zeroize();
        self.renderer.invalidate();
    }
    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        e: KeyEvent,
    ) {
        self.key(e);
    }
    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        e: KeyEvent,
    ) {
        self.key(e);
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        _: KeyEvent,
    ) {
    }
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _: u32,
        m: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.modifiers = m;
    }
}
impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlPointer,
        events: &[PointerEvent],
    ) {
        for e in events {
            if let PointerEventKind::Press { button: 0x110, .. } = e.kind {
                if let Some(v) = self.views.iter().find(|v| v.surface() == &e.surface) {
                    let (x, y, w, h) = render::panel_rect(
                        v.width as f64,
                        v.height as f64,
                        self.config.ui_scale as f64,
                    );
                    let dx = (e.position.0 - x) / w * 800.;
                    let dy = (e.position.1 - y) / h * 580.;
                    if (44.0..395.0).contains(&dx) && (393.0..443.0).contains(&dy) {
                        self.submit();
                    }
                }
            }
        }
    }
}
impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}
smithay_client_toolkit::delegate_registry!(App);

#[cfg(test)]
mod cli_tests {
    use super::*;
    #[test]
    fn timers_cannot_be_used_with_real_lock() {
        assert!(Args::try_parse_from(["motion-lock", "--lock", "--preview-seconds", "1"]).is_err());
        assert!(
            Args::try_parse_from(["motion-lock", "--check", "--preview-seconds", "1"]).is_err()
        );
        assert!(
            Args::try_parse_from(["motion-lock", "--preview", "--preview-seconds", "1"]).is_ok()
        );
    }
    #[test]
    fn mode_is_explicit_and_exclusive() {
        assert!(Args::try_parse_from(["motion-lock"]).is_err());
        assert!(Args::try_parse_from(["motion-lock", "--preview", "--lock"]).is_err());
    }
}

smithay_client_toolkit::delegate_dispatch2!(App);
