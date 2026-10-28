use super::interface::{Access, Interface};
use crate::{
    auth::{Client, Event},
    config::Config,
    widget::{DrawContext, DrawReport, KeyState, ModifiersState, Widget},
};
use greetd_ipc::AuthMessageType;
use xkeysym::key as keysyms;
use zeroize::Zeroize;

pub struct Login {
    question: String,
    answer: String,
    mode: Option<AuthMessageType>,
    error: String,
    interface: Interface,
    dirty: bool,
    client: Option<Client>,
    preview: bool,
    waiting: bool,
    cancelling: bool,
    fingerprint: bool,
    started: bool,
    username: String,
}
impl Login {
    pub fn new(config: &Config) -> Box<Self> {
        let preview = std::env::var_os("MOTION_LOGIN_PREVIEW").is_some();
        let client = if preview {
            None
        } else {
            std::env::var_os("GREETD_SOCK").and_then(|path| {
                Client::new(path.into(), config.command.clone(), config.session_env()).ok()
            })
        };
        let mut login = Self::build(client, preview);
        login.start_default(&config.default_user);
        login
    }
    fn start_default(&mut self, username: &str) {
        if !self.preview && !username.is_empty() {
            self.answer = username.into();
            self.submit();
        }
    }
    fn build(client: Option<Client>, preview: bool) -> Box<Self> {
        Box::new(Self {
            question: "Username:".into(),
            answer: String::new(),
            mode: None,
            error: String::new(),
            interface: Interface::new(),
            dirty: true,
            client,
            preview,
            waiting: false,
            cancelling: false,
            fingerprint: false,
            started: false,
            username: String::new(),
        })
    }
    fn reset(&mut self) {
        self.answer.zeroize();
        self.mode = None;
        self.waiting = false;
        self.cancelling = false;
        self.fingerprint = false;
        self.username.clear();
        self.question = "Username:".into();
        self.dirty = true;
    }
    fn submit(&mut self) {
        if self.waiting || self.cancelling {
            return;
        }
        if self.preview {
            self.answer.zeroize();
            self.error = "Preview: sign in disabled".into();
            self.dirty = true;
            return;
        }
        if self.mode.is_none() && self.answer.trim().is_empty() {
            return;
        }
        let answer = std::mem::take(&mut self.answer);
        if self.mode.is_none() {
            self.username = answer.clone();
        }
        let result = self
            .client
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Authentication service unavailable"))
            .and_then(|client| {
                if self.mode.is_some() {
                    client.answer(answer)
                } else {
                    client.start(answer)
                }
            });
        match result {
            Ok(()) => {
                self.waiting = true;
                self.question = format!("{}: checking authentication ...", self.username);
                self.error.clear();
            }
            Err(error) => {
                self.error = error.to_string();
            }
        }
        self.dirty = true;
    }
    fn cancel(&mut self) {
        if self.cancelling {
            return;
        }
        self.answer.zeroize();
        self.error.clear();
        if self.waiting || self.mode.is_some() {
            if let Some(client) = &self.client {
                if let Err(error) = client.cancel() {
                    self.reset();
                    self.error = error.to_string();
                } else {
                    self.cancelling = true;
                    self.waiting = true;
                    self.question = "Cancelling ...".into();
                }
            } else {
                self.reset();
            }
        } else {
            self.reset();
        }
        self.dirty = true;
    }
}
impl Widget for Login {
    fn poll(&mut self) -> bool {
        let events = self.client.as_ref().map(Client::events).unwrap_or_default();
        let changed = !events.is_empty();
        for event in events {
            match event {
                Event::Prompt(kind, message) => {
                    if !self.cancelling {
                        self.answer.zeroize();
                        self.mode = Some(kind);
                        self.question = format!("{}: {}", self.username, message);
                        self.waiting = false;
                        self.fingerprint = false;
                        self.error.clear();
                    }
                }
                Event::Info { error, message } => {
                    if !self.cancelling {
                        let text = message.to_lowercase();
                        self.fingerprint = text.contains("finger") || text.contains("scan");
                        self.mode = None;
                        self.waiting = true;
                        self.question = format!("{}: {}", self.username, message);
                        if error {
                            self.error = message;
                        } else {
                            self.error.clear();
                        }
                    }
                }
                Event::Failed(message) => {
                    self.reset();
                    self.error = message;
                }
                Event::Cancelled => {
                    self.reset();
                    self.error = "Sign in cancelled".into();
                }
                Event::Started => {
                    self.answer.zeroize();
                    self.started = true;
                }
            }
            self.dirty = true;
        }
        changed
    }
    fn wake_fd(&self) -> Option<i32> {
        self.client.as_ref().map(Client::fd)
    }
    fn authenticated(&self) -> bool {
        self.started
    }
    fn size(&self) -> (u32, u32) {
        (800, 580)
    }
    fn draw(&mut self, ctx: &mut DrawContext, _: (u32, u32)) -> Result<DrawReport, std::io::Error> {
        let (width, height) = self.size();
        if !self.dirty && !ctx.force {
            return Ok(DrawReport::empty(width, height));
        }
        self.dirty = false;
        let mut buf = ctx.buf.subdimensions((0, 0, width, height))?;
        self.interface.draw(
            &mut buf,
            ctx.config,
            Access {
                question: &self.question,
                answer: &self.answer,
                secret: matches!(self.mode, Some(AuthMessageType::Secret)),
                error: &self.error,
                preview: self.preview,
                waiting: self.waiting,
                fingerprint: self.fingerprint,
                cancelling: self.cancelling,
            },
        )?;
        Ok(DrawReport {
            width,
            height,
            damage: vec![buf.get_signed_bounds()],
            full_damage: false,
        })
    }
    fn keyboard_input(
        &mut self,
        key: u32,
        modifiers: ModifiersState,
        _: KeyState,
        interpreted: Option<String>,
    ) {
        if key == keysyms::Escape {
            if self.preview {
                std::process::exit(0);
            }
            self.cancel();
            return;
        }
        if modifiers.ctrl && (key == keysyms::u || key == keysyms::c) {
            self.cancel();
            return;
        }
        if self.waiting || self.cancelling {
            return;
        }
        match key {
            keysyms::BackSpace => {
                self.answer.pop();
                self.dirty = true;
            }
            keysyms::Return => self.submit(),
            _ => {
                if let Some(value) = interpreted {
                    if !value.chars().any(char::is_control) {
                        self.answer += &value;
                        self.dirty = true;
                    }
                }
            }
        }
    }
    fn mouse_click(&mut self, button: u32, (x, y): (u32, u32)) {
        if button == 0x110 && (44..395).contains(&x) && (393..443).contains(&y) {
            if self.waiting {
                self.cancel();
            } else {
                self.submit();
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_user_starts_auth_and_shortcut_returns_to_user_entry() {
        use greetd_ipc::{codec::SyncCodec, Request, Response};
        use std::{os::unix::net::UnixListener, thread};
        let path =
            std::env::temp_dir().join(format!("motion-default-user-{}.sock", std::process::id()));
        let listener = UnixListener::bind(&path).unwrap();
        let server = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            assert!(
                matches!(Request::read_from(&mut s).unwrap(), Request::CreateSession { username } if username == "held")
            );
            Response::AuthMessage {
                auth_message_type: AuthMessageType::Secret,
                auth_message: "Password:".into(),
            }
            .write_to(&mut s)
            .unwrap();
            assert!(matches!(
                Request::read_from(&mut s).unwrap(),
                Request::CancelSession
            ));
            Response::Success.write_to(&mut s).unwrap();
        });
        let client = Client::new(
            path.clone(),
            "niri-session".into(),
            Config::default().session_env(),
        )
        .unwrap();
        let mut login = Login::build(Some(client), false);
        login.start_default("held");
        assert!(login.waiting);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while login.mode.is_none() {
            login.poll();
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(5));
        }
        login.keyboard_input(
            keysyms::u,
            ModifiersState {
                ctrl: true,
                ..Default::default()
            },
            KeyState::Pressed,
            None,
        );
        while login.cancelling {
            login.poll();
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(login.question, "Username:");
        assert!(login.answer.is_empty());
        assert!(login.username.is_empty());
        assert!(!login.waiting);
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        let mut preview = Login::build(None, true);
        preview.start_default("held");
        assert!(preview.answer.is_empty());
    }
    #[test]
    fn preview_isolated_and_unicode_backspace() {
        let mut login = Login::build(None, true);
        login.answer = "ä🙂".into();
        login.keyboard_input(
            keysyms::BackSpace,
            ModifiersState::default(),
            KeyState::Pressed,
            None,
        );
        assert_eq!(login.answer, "ä");
        login.mouse_click(0x110, (100, 420));
        assert!(login.answer.is_empty());
        assert!(login.client.is_none());
        assert!(login.error.contains("Preview"));
    }
}
