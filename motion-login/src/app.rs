//! Current SCTK/wayland-client frontend, with a single event/render thread.
use crate::{
    buffer::Buffer,
    config::{Config, OutputMode},
    panel::{GlitchClock, Panel},
    widget::{DrawContext, KeyState, Widget},
    widgets::login::Login,
};
use anyhow::Result;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers, RepeatInfo},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use std::{
    collections::HashMap,
    os::fd::AsRawFd,
    time::{Duration, Instant},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, Proxy, QueueHandle,
};

struct Screen {
    layer: LayerSurface,
    output: Option<wl_output::WlOutput>,
    configured: bool,
}
struct Repeat {
    event: KeyEvent,
    next: Instant,
}
struct App {
    // One EGL display/context across all outputs; drop before layer surfaces.
    panel: Option<Panel>,
    screens: Vec<Screen>,
    registry: RegistryState,
    outputs: OutputState,
    seats: SeatState,
    compositor: CompositorState,
    layers: LayerShell,
    keyboards: HashMap<wl_seat::WlSeat, wl_keyboard::WlKeyboard>,
    pointers: HashMap<wl_seat::WlSeat, wl_pointer::WlPointer>,
    modifiers: Modifiers,
    repeat: Option<Repeat>,
    repeat_delay: Duration,
    repeat_interval: Option<Duration>,
    widget: Box<dyn Widget>,
    pixels: Vec<u8>,
    config: Config,
    glitch: GlitchClock,
    glitch_visible: bool,
    next_glitch_frame: Instant,
    dirty: bool,
    force: bool,
    exit: bool,
}
pub fn run(config: Config) -> Result<()> {
    config.validate().map_err(anyhow::Error::msg)?;
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<App>(&conn)?;
    let qh = queue.handle();
    let compositor = CompositorState::bind(&globals, &qh)?;
    let layers = LayerShell::bind(&globals, &qh)?;
    let now = Instant::now();
    let mut app = App {
        panel: None,
        screens: vec![],
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        seats: SeatState::new(&globals, &qh),
        compositor,
        layers,
        keyboards: HashMap::new(),
        pointers: HashMap::new(),
        modifiers: Modifiers::default(),
        repeat: None,
        repeat_delay: Duration::from_millis(250),
        repeat_interval: Some(Duration::from_secs_f64(1. / 30.)),
        widget: Login::new(&config),
        pixels: vec![],
        glitch: GlitchClock::new(config.glitch.clone(), now),
        glitch_visible: false,
        next_glitch_frame: now,
        config,
        dirty: true,
        force: true,
        exit: false,
    };
    if matches!(app.config.output_mode, OutputMode::Active) {
        app.add_screen(&qh, None);
    }
    queue.roundtrip(&mut app)?;
    if matches!(app.config.output_mode, OutputMode::All) {
        for output in app.outputs.outputs() {
            app.add_screen(&qh, Some(output));
        }
    }
    while !app.exit {
        queue.dispatch_pending(&mut app)?;
        if app.exit {
            break;
        }
        let timeout = app.tick(&conn)?;
        if app.exit {
            break;
        }
        conn.flush()?;
        let Some(guard) = queue.prepare_read() else {
            continue;
        };
        let mut fds = vec![libc::pollfd {
            fd: guard.connection_fd().as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        }];
        if let Some(fd) = app.widget.wake_fd() {
            fds.push(libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            });
        }
        let result = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as _, timeout) };
        if result < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
            return Err(std::io::Error::last_os_error().into());
        }
        if fds[0].revents & libc::POLLIN != 0 {
            guard.read()?;
        } else {
            drop(guard);
        }
    }
    // Explicitly drop surfaces/GL contexts while the connection is still alive.
    app.panel.take();
    app.screens.clear();
    Ok(())
}
impl App {
    fn add_screen(&mut self, qh: &QueueHandle<Self>, output: Option<wl_output::WlOutput>) {
        if self.screens.iter().any(|s| s.output == output) {
            return;
        }
        self.panel.take();
        let surface = self.compositor.create_surface(qh);
        surface.set_buffer_scale(self.config.scale as i32);
        let layer = self.layers.create_layer_surface(
            qh,
            surface,
            Layer::Overlay,
            Some("motion-login"),
            output.as_ref(),
        );
        let size = self.config.buffer_size(self.widget.size());
        layer.set_size(
            size.0 as u32 / self.config.scale,
            size.1 as u32 / self.config.scale,
        );
        layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer.commit();
        self.screens.push(Screen {
            layer,
            output,
            configured: false,
        });
    }
    fn input(&mut self) {
        self.glitch.input(Instant::now());
        self.dirty = true;
    }
    fn key(&mut self, event: KeyEvent) {
        if std::env::var_os("MOTION_LOGIN_PREVIEW").is_some() && event.keysym == Keysym::F8 {
            self.glitch.trigger(Instant::now());
            return;
        }
        self.input();
        self.widget.keyboard_input(
            event.keysym.raw(),
            self.modifiers,
            KeyState::Pressed,
            event.utf8,
        );
    }
    fn tick(&mut self, conn: &Connection) -> Result<i32> {
        if self.widget.poll() {
            self.dirty = true;
        }
        if self.widget.authenticated() {
            self.exit = true;
            return Ok(0);
        }
        let now = Instant::now();
        if self.repeat.as_ref().is_some_and(|r| now >= r.next) {
            let event = self.repeat.as_ref().unwrap().event.clone();
            if let Some(interval) = self.repeat_interval {
                self.repeat.as_mut().unwrap().next = now + interval;
            } else {
                self.repeat = None;
            }
            self.key(event);
        }
        let design = self.widget.size();
        let output_size = self.config.buffer_size(design);
        if self.dirty {
            self.pixels.resize((design.0 * design.1 * 4) as usize, 0);
            let mut buffer = Buffer::new(&mut self.pixels, design);
            let report = self.widget.draw(
                &mut DrawContext {
                    buf: &mut buffer,
                    force: self.force,
                    config: &self.config,
                },
                (0, 0),
            )?;
            debug_assert_eq!((report.width, report.height), design);
            let changed = self.force
                || report.full_damage
                || !report.damage.is_empty()
                || self.glitch_visible;
            if !self.screens.is_empty() && self.screens.iter().all(|s| s.configured) {
                if self.panel.is_none() {
                    let surfaces: Vec<_> = self
                        .screens
                        .iter()
                        .map(|s| s.layer.wl_surface().clone())
                        .collect();
                    self.panel = Some(Panel::new(conn, &surfaces, output_size)?);
                }
                if changed {
                    self.panel.as_ref().unwrap().draw(
                        Some((&self.pixels, design)),
                        output_size,
                        0.,
                        0.,
                    )?;
                }
            }
            self.glitch_visible = false;
            self.dirty = false;
            self.force = false;
        }
        let (amount, phase, mut timeout) = self.glitch.sample(now);
        if (amount > 0. && now >= self.next_glitch_frame) || (amount == 0. && self.glitch_visible) {
            if let Some(panel) = &self.panel {
                panel.draw(None, output_size, amount, phase)?;
            }
            self.next_glitch_frame = now + Duration::from_millis(33);
            self.glitch_visible = amount > 0.;
        }
        if let Some(repeat) = &self.repeat {
            let delay = repeat
                .next
                .saturating_duration_since(now)
                .as_millis()
                .max(1) as i32;
            timeout = if timeout < 0 {
                delay
            } else {
                timeout.min(delay)
            };
        }
        Ok(timeout)
    }
}
impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: i32,
    ) {
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}
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
        if matches!(self.config.output_mode, OutputMode::All) {
            self.add_screen(qh, Some(output));
        }
    }
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.panel.take();
        self.screens.retain(|s| s.output.as_ref() != Some(&output));
        self.dirty = true;
        self.force = true;
    }
}
impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        self.panel.take();
        self.screens.retain(|s| &s.layer != layer);
        self.dirty = true;
        self.force = true;
        self.exit = self.screens.is_empty();
    }
    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        _: LayerSurfaceConfigure,
        _: u32,
    ) {
        for screen in &mut self.screens {
            if &screen.layer == layer {
                screen.configured = true;
            }
        }
        self.dirty = true;
        self.force = true;
    }
}
impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Keyboard => {
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    self.keyboards.entry(seat.clone())
                {
                    if let Ok(keyboard) = self.seats.get_keyboard(qh, &seat, None) {
                        entry.insert(keyboard);
                    }
                }
            }
            Capability::Pointer => {
                if let std::collections::hash_map::Entry::Vacant(entry) =
                    self.pointers.entry(seat.clone())
                {
                    if let Ok(pointer) = self.seats.get_pointer(qh, &seat) {
                        entry.insert(pointer);
                    }
                }
            }
            _ => {}
        }
    }
    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        match capability {
            Capability::Keyboard => {
                if let Some(k) = self.keyboards.remove(&seat) {
                    k.release();
                }
                self.repeat = None;
            }
            Capability::Pointer => {
                if let Some(p) = self.pointers.remove(&seat) {
                    p.release();
                }
            }
            _ => {}
        }
    }
    fn remove_seat(&mut self, conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.remove_capability(conn, qh, seat.clone(), Capability::Keyboard);
        self.remove_capability(conn, qh, seat, Capability::Pointer);
    }
}
impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }
    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
        self.repeat = None;
        self.modifiers = Modifiers::default();
    }
    fn press_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        let repeatable = repeatable(&event, self.modifiers);
        self.repeat = if keyboard.version() < 10 && repeatable && self.repeat_interval.is_some() {
            Some(Repeat {
                event: event.clone(),
                next: Instant::now() + self.repeat_delay,
            })
        } else {
            None
        };
        self.key(event);
    }
    fn repeat_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if repeatable(&event, self.modifiers) {
            self.key(event);
        }
    }
    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if self
            .repeat
            .as_ref()
            .is_some_and(|r| r.event.raw_code == event.raw_code)
        {
            self.repeat = None;
        }
    }
    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: Modifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.modifiers = modifiers;
        self.repeat = None;
    }
    fn update_repeat_info(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        info: RepeatInfo,
    ) {
        match info {
            RepeatInfo::Repeat { rate, delay } => {
                self.repeat_interval = Some(Duration::from_secs_f64(1. / rate.get() as f64));
                self.repeat_delay = Duration::from_millis(delay.into());
            }
            RepeatInfo::Disable => {
                self.repeat_interval = None;
                self.repeat = None;
            }
        }
    }
}
impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if !self
                .screens
                .iter()
                .any(|s| s.layer.wl_surface() == &event.surface)
            {
                continue;
            }
            if let PointerEventKind::Release { button, .. } = event.kind {
                let position = (
                    (event.position.0 * self.config.scale as f64) as u32,
                    (event.position.1 * self.config.scale as f64) as u32,
                );
                self.input();
                self.widget
                    .mouse_click(button, self.config.design_position(position));
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
delegate_registry!(App);
smithay_client_toolkit::delegate_dispatch2!(App);

fn repeatable(event: &KeyEvent, modifiers: Modifiers) -> bool {
    !modifiers.ctrl
        && !modifiers.alt
        && (event.keysym == Keysym::BackSpace
            || event
                .utf8
                .as_ref()
                .is_some_and(|text| !text.is_empty() && !text.chars().any(char::is_control)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeating_enter_never_submits_credentials_again() {
        let mut event = KeyEvent {
            time: 0,
            raw_code: 28,
            keysym: Keysym::Return,
            utf8: Some("\r".into()),
        };
        assert!(!repeatable(&event, Modifiers::default()));
        event.keysym = Keysym::a;
        event.utf8 = Some("a".into());
        assert!(repeatable(&event, Modifiers::default()));
        assert!(!repeatable(
            &event,
            Modifiers {
                ctrl: true,
                ..Modifiers::default()
            }
        ));
        event.keysym = Keysym::BackSpace;
        event.utf8 = None;
        assert!(repeatable(&event, Modifiers::default()));
    }
}
