use crate::{
    config::{Config, Kind, Period},
    egl,
    gpu::Gpu,
    ipc, niri, power,
    selection::{self, RotationClock},
    theme,
    video::Video,
    wake::Wake,
};
use anyhow::{bail, Context, Result};
use glow::HasContext;
use serde_json::json;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData, Region},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{Capability, SeatHandler, SeatState},
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use std::{
    collections::HashSet,
    io::{BufRead, BufReader, Write},
    os::fd::AsRawFd,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_seat, wl_surface},
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::{
    ext::idle_notify::v1::client::{
        ext_idle_notification_v1 as idle_notification, ext_idle_notifier_v1 as idle_manager,
    },
    wp::{
        fractional_scale::v1::client::{
            wp_fractional_scale_manager_v1 as frac_manager, wp_fractional_scale_v1 as frac,
        },
        viewporter::client::{wp_viewport, wp_viewporter},
    },
};

static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: i32) {
    STOP.store(true, Ordering::Relaxed);
}
enum Renderer {
    Gpu(Gpu),
    Video(Video),
}
impl Renderer {
    fn new(
        kind: Kind,
        path: &std::path::Path,
        gl: &glow::Context,
        config: &Config,
        _display: &egl::Display,
        conn: &Connection,
        wake: Arc<Wake>,
    ) -> Result<Self> {
        Ok(match kind {
            Kind::Shader => Self::Gpu(Gpu::shader(
                gl,
                path,
                config.wallpapers[config.index()].stabilize,
            )?),
            Kind::Image => Self::Gpu(Gpu::image(gl, path)?),
            Kind::Video => Self::Video(Video::new(
                path,
                config,
                conn.backend().display_ptr().cast(),
                wake,
            )?),
        })
    }
    fn destroy(self, gl: &glow::Context) {
        if let Self::Gpu(gpu) = self {
            gpu.destroy(gl);
        }
    }
}
struct Screen {
    output: wl_output::WlOutput,
    name: String,
    layer: LayerSurface,
    viewport: wp_viewport::WpViewport,
    fractional: Option<frac::WpFractionalScaleV1>,
    logical: (u32, u32),
    scale: f64,
    pixels: (i32, i32),
    configured: bool,
    dirty: bool,
    pending: Option<Instant>,
    next_frame: Instant,
    frames: u64,
    renderer: Option<Renderer>,
    surface: Option<egl::Surface>,
}
pub struct App {
    screens: Vec<Screen>,
    egl: egl::Display,
    gl: Option<glow::Context>,
    registry: RegistryState,
    outputs: OutputState,
    seats: SeatState,
    compositor: CompositorState,
    layers: LayerShell,
    viewporter: wp_viewporter::WpViewporter,
    fractional: Option<frac_manager::WpFractionalScaleManagerV1>,
    idle_manager: Option<idle_manager::ExtIdleNotifierV1>,
    idle_notifications: Vec<(wl_seat::WlSeat, idle_notification::ExtIdleNotificationV1)>,
    idle_seats: HashSet<wl_seat::WlSeat>,
    config: Config,
    path: PathBuf,
    conn: Connection,
    wake: Arc<Wake>,
    exit: bool,
    manual_pause: bool,
    battery: bool,
    next_battery: Instant,
    fullscreen: niri::VisibleTiles,
    niri: mpsc::Receiver<niri::VisibleTiles>,
    animation: f64,
    last_tick: Instant,
    clock_running: bool,
    rotation: RotationClock,
    period: Period,
    mode_override: Option<Period>,
    next_mode_check: Instant,
    theme_sample: Option<Period>,
    theme_rx: mpsc::Receiver<Option<Period>>,
    theme_enabled: Arc<AtomicBool>,
}
pub fn run(path: PathBuf) -> Result<()> {
    let mut config = Config::load(&path)?;
    let period = selection::resolve(&config, None, None, selection::local_minute());
    if !config.eligible(period).contains(&config.index()) {
        config.active = config.wallpapers[config.eligible(period)[0]].name.clone();
    }
    let server = ipc::Server::bind()?;
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<App>(&conn)?;
    let qh = queue.handle();
    let compositor = CompositorState::bind(&globals, &qh)?;
    let layers = LayerShell::bind(&globals, &qh)?;
    let viewporter = globals
        .bind(&qh, 1..=1, ())
        .context("wp_viewporter required")?;
    let fractional = globals.bind(&qh, 1..=1, ()).ok();
    let idle_manager = globals.bind(&qh, 1..=2, ()).ok();
    let egl = egl::Display::new(&conn)?;
    let wake = Arc::new(Wake::new()?);
    let niri = niri::watch(wake.clone());
    let theme_enabled = Arc::new(AtomicBool::new(
        config.day_night.enabled && config.day_night.source == crate::config::ThemeSource::Noctalia,
    ));
    let theme_rx = theme::watch(wake.clone(), theme_enabled.clone());
    let rotation = RotationClock::new(Instant::now(), config.rotation.interval_seconds);
    if config.pause_on_idle && idle_manager.is_none() {
        log::warn!("ext-idle-notify unavailable: idle pause disabled");
    }
    let mut app = App {
        screens: vec![],
        egl,
        gl: None,
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        seats: SeatState::new(&globals, &qh),
        compositor,
        layers,
        viewporter,
        fractional,
        idle_manager,
        idle_notifications: vec![],
        idle_seats: HashSet::new(),
        config,
        path,
        conn: conn.clone(),
        wake,
        exit: false,
        manual_pause: false,
        battery: power::on_battery(),
        next_battery: Instant::now() + Duration::from_secs(30),
        fullscreen: niri::VisibleTiles::new(),
        niri,
        animation: 0.,
        last_tick: Instant::now(),
        clock_running: false,
        rotation,
        period,
        mode_override: None,
        next_mode_check: Instant::now(),
        theme_sample: None,
        theme_rx,
        theme_enabled,
    };
    queue.roundtrip(&mut app)?;
    for output in app.outputs.outputs() {
        app.add_output(&qh, output)?;
    }
    app.setup_idle(&qh);
    unsafe {
        libc::signal(libc::SIGINT, stop as *const () as usize);
        libc::signal(libc::SIGTERM, stop as *const () as usize);
    }
    log::info!(
        "native Wayland background started; {} FPS (battery {}), render scale {}",
        app.config.fps,
        app.config.battery_fps,
        app.config.render_scale
    );
    while !app.exit && !STOP.load(Ordering::Relaxed) {
        queue.dispatch_pending(&mut app)?;
        while let Ok(fullscreen) = app.niri.try_recv() {
            app.fullscreen = fullscreen;
        }
        loop {
            match server.listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_read_timeout(Some(Duration::from_millis(100)))?;
                    stream.set_write_timeout(Some(Duration::from_millis(100)))?;
                    let mut line = String::new();
                    use std::io::Read;
                    let result = BufReader::new((&mut stream).take(4096))
                        .read_line(&mut line)
                        .map_err(anyhow::Error::from)
                        .and_then(|_| app.command(line.trim(), &qh));
                    let response = match result {
                        Ok(v) => v,
                        Err(e) => json!({"error":format!("{e:#}")}),
                    };
                    let _ = writeln!(stream, "{response}");
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        let timeout = app.tick(&qh)?;
        conn.flush()?;
        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        let mut fds = [
            libc::pollfd {
                fd: guard.connection_fd().as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: server.listener.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: app.wake.fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        let result = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, timeout) };
        if result < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return Err(std::io::Error::last_os_error().into());
        }
        if fds[0].revents & libc::POLLIN != 0 {
            guard.read()?;
        } else {
            drop(guard);
        }
        if fds[2].revents & libc::POLLIN != 0 {
            app.wake.drain();
        }
    }
    Ok(())
}
impl App {
    fn add_output(&mut self, qh: &QueueHandle<Self>, output: wl_output::WlOutput) -> Result<()> {
        if self.screens.iter().any(|s| s.output == output) {
            return Ok(());
        }
        let info = self.outputs.info(&output);
        let name = info
            .as_ref()
            .and_then(|i| i.name.clone())
            .unwrap_or_else(|| format!("output-{}", output.id().protocol_id()));
        let surface = self.compositor.create_surface(qh);
        let region = Region::new(&self.compositor)?;
        surface.set_input_region(Some(region.wl_region()));
        let viewport = self.viewporter.get_viewport(&surface, qh, ());
        let fractional = self
            .fractional
            .as_ref()
            .map(|m| m.get_fractional_scale(&surface, qh, surface.clone()));
        let layer = self.layers.create_layer_surface(
            qh,
            surface,
            Layer::Background,
            Some("niri-motion-background"),
            Some(&output),
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(0, 0);
        layer.commit();
        self.screens.push(Screen {
            output,
            name,
            layer,
            viewport,
            fractional,
            logical: (0, 0),
            scale: info.map_or(1., |i| i.scale_factor as f64),
            pixels: (0, 0),
            configured: false,
            dirty: true,
            pending: None,
            next_frame: Instant::now(),
            frames: 0,
            renderer: None,
            surface: None,
        });
        Ok(())
    }
    fn setup_idle(&mut self, qh: &QueueHandle<Self>) {
        for (_, n) in self.idle_notifications.drain(..) {
            n.destroy();
        }
        self.idle_seats.clear();
        if !self.config.pause_on_idle {
            return;
        }
        if let Some(m) = &self.idle_manager {
            for seat in self.seats.seats() {
                let n = m.get_idle_notification(
                    self.config.idle_seconds.saturating_mul(1000),
                    &seat,
                    qh,
                    seat.clone(),
                );
                self.idle_notifications.push((seat, n));
            }
        }
    }
    fn tick(&mut self, qh: &QueueHandle<Self>) -> Result<i32> {
        let now = Instant::now();
        self.selection_tick(qh, now);
        if now >= self.next_battery {
            let battery = power::on_battery();
            if battery != self.battery {
                log::info!("power source: {}", if battery { "battery" } else { "AC" });
                self.battery = battery;
                for s in &mut self.screens {
                    s.dirty = true;
                }
            }
            self.next_battery = now + Duration::from_secs(30);
        }
        let global_pause = self.manual_pause
            || !self.idle_seats.is_empty()
            || (self.battery && self.config.pause_on_battery);
        let elapsed = now.duration_since(self.last_tick);
        self.last_tick = now;
        let any_active = self
            .screens
            .iter()
            .any(|s| s.configured && !self.paused(s, now, global_pause));
        let interval = Duration::from_secs_f64(1. / self.config.frame_rate(self.battery) as f64);
        self.animation +=
            crate::pacing::animation_step(elapsed, self.clock_running, any_active, interval)
                .as_secs_f64();
        self.clock_running = any_active;
        let scale = self.config.effective_scale(self.battery);
        let mut deadline = self.next_battery.min(self.next_mode_check);
        if self.config.rotation.enabled && self.rotation.due > now {
            deadline = deadline.min(self.rotation.due);
        }
        for i in 0..self.screens.len() {
            if !self.screens[i].configured {
                continue;
            }
            let paused = self.paused(&self.screens[i], now, global_pause);
            let s = &mut self.screens[i];
            let pixels = (
                (s.logical.0 as f64 * s.scale * scale).round().max(1.) as i32,
                (s.logical.1 as f64 * s.scale * scale).round().max(1.) as i32,
            );
            if s.surface.is_none() {
                let surface = self.egl.surface(s.layer.wl_surface(), pixels.0, pixels.1)?;
                self.egl.current(&surface)?;
                if self.gl.is_none() {
                    let gl = self.egl.gl();
                    unsafe {
                        log::info!(
                            "GPU: {} / {}",
                            gl.get_parameter_string(glow::RENDERER),
                            gl.get_parameter_string(glow::VERSION)
                        );
                    }
                    self.gl = Some(gl);
                }
                s.surface = Some(surface);
            }
            let surface = s.surface.as_ref().unwrap();
            self.egl.current(surface)?;
            let gl = self.gl.as_ref().unwrap();
            if pixels != s.pixels {
                surface.resize(pixels.0, pixels.1);
                s.viewport
                    .set_destination(s.logical.0 as i32, s.logical.1 as i32);
                s.pixels = pixels;
                s.dirty = true;
            }
            if s.renderer.is_none() {
                let wp = &self.config.wallpapers[self.config.index()];
                s.renderer = Some(Renderer::new(
                    wp.kind,
                    &wp.source,
                    gl,
                    &self.config,
                    &self.egl,
                    &self.conn,
                    self.wake.clone(),
                )?);
            }
            let wp = &self.config.wallpapers[self.config.index()];
            let kind = wp.kind;
            let speed = wp.speed;
            let brightness = wp.brightness
                * match self.period {
                    Period::Night => self.config.day_night.night_brightness,
                    Period::Day => self.config.day_night.day_brightness,
                    Period::Any => 1.,
                };
            let renderer = s.renderer.as_mut().unwrap();
            if let Renderer::Video(v) = renderer {
                v.pause(paused || v.failed)?;
                if let Err(e) = v.update() {
                    log::error!("{}: {e:#}", s.name);
                }
            }
            let wants = match renderer {
                Renderer::Gpu(_) => (kind == Kind::Shader && speed > 0.) || s.dirty,
                Renderer::Video(v) => !v.failed && (v.pending() || s.dirty),
            };
            if let Some(pending) = s.pending {
                if self.config.pause_when_hidden && matches!(renderer, Renderer::Video(_)) {
                    let hide_at = pending + Duration::from_millis(self.config.hidden_timeout_ms);
                    if hide_at > now {
                        deadline = deadline.min(hide_at);
                    }
                }
                continue;
            }
            // Draw an initial still even if paused, but do not keep animating it.
            if wants && (!paused || s.frames == 0) {
                let due = s.next_frame;
                if now >= due {
                    match renderer {
                        Renderer::Gpu(gpu) => gpu.draw(
                            gl,
                            pixels.0,
                            pixels.1,
                            self.animation as f32 * speed,
                            self.period == Period::Night,
                            brightness,
                        )?,
                        Renderer::Video(v) => v.draw(pixels.0, pixels.1)?,
                    }
                    s.layer
                        .wl_surface()
                        .frame(qh, FrameCallbackData(s.layer.wl_surface().clone()));
                    s.pending = Some(now);
                    surface.swap()?;
                    if let Renderer::Video(v) = renderer {
                        v.swapped();
                    }
                    s.next_frame = crate::pacing::next_deadline(due, interval, Instant::now());
                    s.frames += 1;
                    s.dirty = false;
                    // The next frame callback wakes the loop; no timer until then.
                } else {
                    deadline = deadline.min(due);
                }
            }
        }
        Ok(deadline
            .saturating_duration_since(Instant::now())
            .as_millis()
            .saturating_add(1)
            .min(i32::MAX as u128) as i32)
    }
    fn paused(&self, s: &Screen, now: Instant, global: bool) -> bool {
        global
            || (self.config.pause_on_fullscreen
                && self.fullscreen.get(&s.name).is_some_and(|tiles| {
                    tiles.iter().any(|(w, h)| {
                        *w >= s.logical.0 as f64 - 0.5 && *h >= s.logical.1 as f64 - 0.5
                    })
                }))
            || (self.config.pause_when_hidden
                && s.pending.is_some_and(|t| {
                    now.duration_since(t) >= Duration::from_millis(self.config.hidden_timeout_ms)
                }))
    }
    fn apply(&mut self, candidate: Config, qh: &QueueHandle<Self>) -> Result<()> {
        let mut staged: Vec<(usize, Renderer)> = vec![];
        let wp = &candidate.wallpapers[candidate.index()];
        for (i, s) in self.screens.iter().enumerate() {
            if let Some(surface) = &s.surface {
                self.egl.current(surface)?;
                let result = Renderer::new(
                    wp.kind,
                    &wp.source,
                    self.gl.as_ref().unwrap(),
                    &candidate,
                    &self.egl,
                    &self.conn,
                    self.wake.clone(),
                );
                match result {
                    Ok(r) => staged.push((i, r)),
                    Err(e) => {
                        for (j, r) in staged {
                            self.egl
                                .current(self.screens[j].surface.as_ref().unwrap())?;
                            r.destroy(self.gl.as_ref().unwrap());
                        }
                        return Err(e);
                    }
                }
            }
        }
        for (i, new) in staged {
            let s = &mut self.screens[i];
            self.egl.current(s.surface.as_ref().unwrap())?;
            if let Some(old) = s.renderer.replace(new) {
                old.destroy(self.gl.as_ref().unwrap());
            }
            s.dirty = true;
            s.frames = 0;
            s.next_frame = Instant::now();
        }
        self.rotation
            .reset(Instant::now(), candidate.rotation.interval_seconds);
        self.next_mode_check = Instant::now();
        self.theme_enabled.store(
            candidate.day_night.enabled
                && candidate.day_night.source == crate::config::ThemeSource::Noctalia
                && self.mode_override.is_none(),
            Ordering::Relaxed,
        );
        self.config = candidate;
        self.animation = 0.;
        self.setup_idle(qh);
        Ok(())
    }
    fn advance(&mut self, backwards: bool, qh: &QueueHandle<Self>) -> Result<()> {
        let mut cursor = self.config.clone();
        let count = cursor.eligible(self.period).len();
        for _ in 0..count {
            let index = selection::adjacent(&cursor, self.period, backwards)
                .context("no eligible wallpapers")?;
            cursor.active = cursor.wallpapers[index].name.clone();
            if cursor.active == self.config.active {
                return Ok(());
            }
            match self.apply(cursor.clone(), qh) {
                Ok(()) => return Ok(()),
                Err(e) => log::warn!("skip {}: {e:#}", cursor.active),
            }
        }
        bail!("no usable wallpaper in the current playlist")
    }
    fn selection_tick(&mut self, qh: &QueueHandle<Self>, now: Instant) {
        while let Ok(sample) = self.theme_rx.try_recv() {
            self.theme_sample = sample;
            self.next_mode_check = now;
        }
        if now >= self.next_mode_check {
            let next = selection::resolve(
                &self.config,
                self.mode_override,
                self.theme_sample,
                selection::local_minute(),
            );
            if next != self.period {
                log::info!("background mode: {next:?}");
                self.period = next;
                for s in &mut self.screens {
                    s.dirty = true;
                }
                if !self.config.eligible(next).contains(&self.config.index()) {
                    if let Err(e) = self.advance(false, qh) {
                        log::error!("day/night switch: {e:#}");
                    }
                }
            }
            self.next_mode_check = now + Duration::from_secs(30);
        }
        if self.config.rotation.enabled && now >= self.rotation.due {
            let global = self.manual_pause
                || !self.idle_seats.is_empty()
                || (self.battery && self.config.pause_on_battery);
            let inactive = self.screens.is_empty()
                || self
                    .screens
                    .iter()
                    .all(|s| !s.configured || self.paused(s, now, global));
            if !self.config.rotation.defer_while_paused || !inactive {
                if let Err(e) = self.advance(false, qh) {
                    log::error!("automatic switch: {e:#}");
                }
                self.rotation
                    .reset(now, self.config.rotation.interval_seconds);
            }
        }
    }
    fn command(&mut self, line: &str, qh: &QueueHandle<Self>) -> Result<serde_json::Value> {
        let (command, arg) = line.split_once(' ').unwrap_or((line, ""));
        match command {
            "status" => {
                return Ok(
                    json!({"wallpaper":self.config.active,"fps":self.config.frame_rate(self.battery),"period":self.period,"mode_override":self.mode_override,"auto_switch":self.config.rotation.enabled,"interval_seconds":self.config.rotation.interval_seconds,"next_switch_seconds":self.rotation.due.saturating_duration_since(Instant::now()).as_secs(),"render_scale":self.config.effective_scale(self.battery),"battery":self.battery,"manual_pause":self.manual_pause,"idle":!self.idle_seats.is_empty(),"outputs":self.screens.iter().map(|s|json!({"name":s.name,"logical_size":s.logical,"buffer_size":s.pixels,"frames":s.frames,"paused":self.paused(s,Instant::now(),self.manual_pause||!self.idle_seats.is_empty()||(self.battery&&self.config.pause_on_battery)),"decoder":match &s.renderer{Some(Renderer::Video(v))=>Some(v.decoder.as_str()),_=>None},"video_failed":matches!(&s.renderer,Some(Renderer::Video(v))if v.failed)})).collect::<Vec<_>>()}),
                )
            }
            "list" => {
                return Ok(
                    json!({"wallpapers":self.config.wallpapers.iter().enumerate().map(|(i,w)|json!({"name":w.name,"type":w.kind,"enabled":w.enabled,"period":w.period,"speed":w.speed,"brightness":w.brightness,"fps":w.fps,"render_scale":w.render_scale,"note":w.note,"stabilize":w.stabilize,"eligible":self.config.eligible(self.period).contains(&i),"active":w.name==self.config.active,"source":w.source})).collect::<Vec<_>>()}),
                )
            }
            "mode" => {
                self.mode_override = match arg {
                    "auto" => None,
                    "day" => Some(Period::Day),
                    "night" => Some(Period::Night),
                    _ => bail!("mode must be auto, day or night"),
                };
                self.theme_enabled.store(
                    self.config.day_night.enabled
                        && self.config.day_night.source == crate::config::ThemeSource::Noctalia
                        && self.mode_override.is_none(),
                    Ordering::Relaxed,
                );
                self.next_mode_check = Instant::now();
                self.selection_tick(qh, Instant::now());
            }
            "auto-switch" => {
                self.config.rotation.enabled = match arg {
                    "on" => true,
                    "off" => false,
                    "toggle" => !self.config.rotation.enabled,
                    _ => bail!("auto-switch must be on, off or toggle"),
                };
                self.rotation
                    .reset(Instant::now(), self.config.rotation.interval_seconds);
            }
            "pause" => self.manual_pause = true,
            "resume" => self.manual_pause = false,
            "toggle" => self.manual_pause = !self.manual_pause,
            "quit" => self.exit = true,
            "reload" => {
                let mut c = Config::load(&self.path)?;
                let period = selection::resolve(
                    &c,
                    self.mode_override,
                    self.theme_sample,
                    selection::local_minute(),
                );
                if !c.eligible(period).contains(&c.index()) {
                    let i = *c
                        .eligible(period)
                        .first()
                        .context("no eligible wallpaper")?;
                    c.active = c.wallpapers[i].name.clone();
                }
                self.apply(c, qh)?;
            }
            "next" | "prev" => {
                self.advance(command == "prev", qh)?;
                self.rotation
                    .reset(Instant::now(), self.config.rotation.interval_seconds);
            }
            "set" => {
                let mut c = self.config.clone();
                let index = c
                    .wallpapers
                    .iter()
                    .position(|w| w.name == arg && w.enabled)
                    .context("unknown or disabled wallpaper name")?;
                c.active = c.wallpapers[index].name.clone();
                self.apply(c, qh)?;
            }
            _ => bail!("unknown command: {command}"),
        }
        Ok(json!({"ok":true}))
    }
    fn remove_output(&mut self, output: &wl_output::WlOutput) {
        if let Some(i) = self.screens.iter().position(|s| &s.output == output) {
            let mut s = self.screens.remove(i);
            if let Some(surface) = &s.surface {
                let _ = self.egl.current(surface);
                if let (Some(r), Some(gl)) = (s.renderer.take(), &self.gl) {
                    r.destroy(gl);
                }
            }
            s.surface.take();
            if let Some(f) = s.fractional.take() {
                f.destroy();
            }
            s.viewport.destroy();
        }
    }
}
impl Drop for App {
    fn drop(&mut self) {
        for mut s in self.screens.drain(..) {
            if let Some(surface) = &s.surface {
                let _ = self.egl.current(surface);
                if let (Some(r), Some(gl)) = (s.renderer.take(), &self.gl) {
                    r.destroy(gl);
                }
            }
            s.surface.take();
            if let Some(f) = s.fractional.take() {
                f.destroy();
            }
            s.viewport.destroy();
        }
    }
}
impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        factor: i32,
    ) {
        for s in &mut self.screens {
            if s.layer.wl_surface() == surface && s.fractional.is_none() {
                s.scale = factor as f64;
                s.dirty = true;
            }
        }
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        for s in &mut self.screens {
            if s.layer.wl_surface() == surface {
                s.pending = None;
            }
        }
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}
impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if let Err(e) = self.add_output(qh, output) {
            log::error!("add output: {e:#}");
            self.exit = true;
        }
    }
    fn update_output(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.outputs.info(&output) {
            for s in &mut self.screens {
                if s.output == output {
                    if let Some(name) = &info.name {
                        s.name = name.clone();
                    }
                    if s.fractional.is_none() {
                        s.scale = info.scale_factor as f64;
                    }
                    s.dirty = true;
                }
            }
        }
    }
    fn output_destroyed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.remove_output(&output);
    }
}
impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if let Some(output) = self
            .screens
            .iter()
            .find(|s| s.layer == *layer)
            .map(|s| s.output.clone())
        {
            self.remove_output(&output);
        }
    }
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        c: LayerSurfaceConfigure,
        _: u32,
    ) {
        for s in &mut self.screens {
            if s.layer == *layer && c.new_size.0 > 0 && c.new_size.1 > 0 {
                s.logical = c.new_size;
                s.configured = true;
                s.dirty = true;
            }
        }
    }
}
impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }
    fn new_seat(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        self.setup_idle(qh);
    }
    fn new_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _: Capability,
    ) {
    }
    fn remove_seat(&mut self, _: &Connection, qh: &QueueHandle<Self>, _: wl_seat::WlSeat) {
        self.setup_idle(qh);
    }
}
impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}
delegate_registry!(App);
smithay_client_toolkit::delegate_dispatch2!(App);
wayland_client::delegate_noop!(App:ignore wp_viewporter::WpViewporter);
wayland_client::delegate_noop!(App:ignore wp_viewport::WpViewport);
wayland_client::delegate_noop!(App:ignore frac_manager::WpFractionalScaleManagerV1);
wayland_client::delegate_noop!(App:ignore idle_manager::ExtIdleNotifierV1);
impl Dispatch<frac::WpFractionalScaleV1, wl_surface::WlSurface> for App {
    fn event(
        state: &mut Self,
        _: &frac::WpFractionalScaleV1,
        event: frac::Event,
        data: &wl_surface::WlSurface,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let frac::Event::PreferredScale { scale } = event {
            for s in &mut state.screens {
                if s.layer.wl_surface() == data {
                    s.scale = scale as f64 / 120.;
                    s.dirty = true;
                }
            }
        }
    }
}
impl Dispatch<idle_notification::ExtIdleNotificationV1, wl_seat::WlSeat> for App {
    fn event(
        state: &mut Self,
        _: &idle_notification::ExtIdleNotificationV1,
        event: idle_notification::Event,
        seat: &wl_seat::WlSeat,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            idle_notification::Event::Idled => {
                state.idle_seats.insert(seat.clone());
            }
            idle_notification::Event::Resumed => {
                state.idle_seats.remove(seat);
            }
            _ => {}
        }
    }
}
