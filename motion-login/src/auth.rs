//! Serialized greetd IPC on a worker: no PAM/socket wait ever runs on the UI thread.
use anyhow::{bail, Context, Result};
use greetd_ipc::{codec::SyncCodec, AuthMessageType, Request, Response};
use motion_render::wake::Wake;
use std::{
    os::unix::net::UnixStream,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};
use zeroize::{Zeroize, Zeroizing};

#[derive(Debug)]
pub enum Event {
    Prompt(AuthMessageType, String),
    Info { error: bool, message: String },
    Failed(String),
    Cancelled,
    Started,
}
enum Command {
    Start(String),
    Answer(Zeroizing<String>),
    Cancel,
}
pub struct Client {
    tx: mpsc::Sender<Command>,
    rx: mpsc::Receiver<Event>,
    wake: Arc<Wake>,
    cancelled: Arc<AtomicBool>,
    socket: Arc<Mutex<Option<UnixStream>>>,
}
impl Client {
    pub fn new(path: PathBuf, command: String, env: Vec<String>) -> Result<Self> {
        let (tx, commands) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        let wake = Arc::new(Wake::new()?);
        let cancelled = Arc::new(AtomicBool::new(false));
        let socket = Arc::new(Mutex::new(None));
        let notify = wake.clone();
        let cancel = cancelled.clone();
        let shared = socket.clone();
        thread::Builder::new()
            .name("greetd-auth".into())
            .spawn(move || {
                let mut stream = None;
                while let Ok(cmd) = commands.recv() {
                    if matches!(cmd, Command::Cancel) && !cancel.load(Ordering::Acquire) {
                        continue;
                    }
                    let result = (|| -> Result<()> {
                        if stream.is_none() {
                            let connection = UnixStream::connect(&path)
                                .context("Authentication service unavailable")?;
                            connection.set_read_timeout(Some(Duration::from_secs(120)))?;
                            connection.set_write_timeout(Some(Duration::from_secs(5)))?;
                            *shared.lock().unwrap() = Some(connection.try_clone()?);
                            stream = Some(connection);
                        }
                        let s = stream.as_mut().unwrap();
                        let request = match cmd {
                            Command::Start(username) => Request::CreateSession { username },
                            Command::Answer(mut answer) => {
                                let value = std::mem::take(&mut *answer);
                                Request::PostAuthMessageResponse {
                                    response: Some(value),
                                }
                            }
                            Command::Cancel => {
                                cancel.store(false, Ordering::Release);
                                Request::CancelSession
                            }
                        };
                        let cancelling = matches!(request, Request::CancelSession);
                        let response = exchange(s, request)?;
                        if cancelling {
                            expect_success(response)?;
                            emit(&events, &notify, Event::Cancelled);
                        } else {
                            conversation(s, response, &command, &env, &cancel, &events, &notify)?;
                        }
                        Ok(())
                    })();
                    if let Err(error) = result {
                        // Do not leave a PAM conversation pending after rejection/transport failure.
                        if let Some(s) = stream.as_mut() {
                            let _ = exchange(s, Request::CancelSession);
                        }
                        cancel.store(false, Ordering::Release);
                        emit(&events, &notify, Event::Failed(error.to_string()));
                    }
                }
            })?;
        Ok(Self {
            tx,
            rx,
            wake,
            cancelled,
            socket,
        })
    }
    pub fn start(&self, username: String) -> Result<()> {
        self.tx.send(Command::Start(username))?;
        Ok(())
    }
    pub fn answer(&self, value: String) -> Result<()> {
        self.tx.send(Command::Answer(Zeroizing::new(value)))?;
        Ok(())
    }
    pub fn cancel(&self) -> Result<()> {
        self.cancelled.store(true, Ordering::Release);
        self.tx.send(Command::Cancel)?;
        Ok(())
    }
    pub fn fd(&self) -> i32 {
        self.wake.fd()
    }
    pub fn events(&self) -> Vec<Event> {
        self.wake.drain();
        self.rx.try_iter().collect()
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        if let Some(socket) = self.socket.lock().unwrap().take() {
            let _ = socket.shutdown(std::net::Shutdown::Both);
        }
    }
}
fn emit(tx: &mpsc::Sender<Event>, wake: &Wake, event: Event) {
    let _ = tx.send(event);
    wake.notify();
}
fn exchange(stream: &mut UnixStream, mut request: Request) -> Result<Response> {
    let sent = request.write_to(stream);
    if let Request::PostAuthMessageResponse {
        response: Some(secret),
    } = &mut request
    {
        secret.zeroize();
    }
    sent?;
    Ok(Response::read_from(stream)?)
}
fn expect_success(response: Response) -> Result<()> {
    match response {
        Response::Success => Ok(()),
        Response::Error { description, .. } => bail!("{description}"),
        _ => bail!("Unerwartete Antwort des Anmeldedienstes"),
    }
}
fn conversation(
    stream: &mut UnixStream,
    mut response: Response,
    command: &str,
    env: &[String],
    cancel: &AtomicBool,
    events: &mpsc::Sender<Event>,
    wake: &Wake,
) -> Result<()> {
    loop {
        // Requests remain strictly serial. A cancellation during a scan is honored
        // before handling its next reply, so a late success can never start a session.
        if cancel.swap(false, Ordering::AcqRel) {
            expect_success(exchange(stream, Request::CancelSession)?)?;
            emit(events, wake, Event::Cancelled);
            return Ok(());
        }
        match response {
            Response::AuthMessage {
                auth_message_type,
                auth_message,
            } => match auth_message_type {
                AuthMessageType::Info | AuthMessageType::Error => {
                    emit(
                        events,
                        wake,
                        Event::Info {
                            error: matches!(auth_message_type, AuthMessageType::Error),
                            message: auth_message,
                        },
                    );
                    response =
                        exchange(stream, Request::PostAuthMessageResponse { response: None })?;
                }
                AuthMessageType::Visible | AuthMessageType::Secret => {
                    emit(events, wake, Event::Prompt(auth_message_type, auth_message));
                    return Ok(());
                }
            },
            Response::Success => {
                expect_success(exchange(
                    stream,
                    Request::StartSession {
                        cmd: vec![command.into()],
                        env: env.to_vec(),
                    },
                )?)?;
                emit(events, wake, Event::Started);
                return Ok(());
            }
            Response::Error { description, .. } => bail!("{description}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::net::UnixListener, time::Instant};
    fn fixture(label: &str) -> (PathBuf, UnixListener) {
        let path =
            std::env::temp_dir().join(format!("motion-auth-{}-{label}.sock", std::process::id()));
        let listener = UnixListener::bind(&path).unwrap();
        (path, listener)
    }
    fn wait(client: &Client, predicate: impl Fn(&Event) -> bool) -> Vec<Event> {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut events = vec![];
        while Instant::now() < deadline {
            events.extend(client.events());
            if events.iter().any(&predicate) {
                return events;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("missing auth event: {events:?}");
    }
    fn message(stream: &mut UnixStream, kind: AuthMessageType, text: &str) {
        Response::AuthMessage {
            auth_message_type: kind,
            auth_message: text.into(),
        }
        .write_to(stream)
        .unwrap();
    }
    #[test]
    fn fingerprint_info_ack_and_password_fallback_then_session() {
        let (path, listener) = fixture("fallback");
        let server = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            assert!(
                matches!(Request::read_from(&mut s).unwrap(),Request::CreateSession {username} if username=="test-user")
            );
            message(
                &mut s,
                AuthMessageType::Info,
                "Place your finger on the sensor",
            );
            assert!(matches!(
                Request::read_from(&mut s).unwrap(),
                Request::PostAuthMessageResponse { response: None }
            ));
            // A slow PAM scan must not stop the UI from receiving the info event.
            thread::sleep(Duration::from_millis(120));
            message(&mut s, AuthMessageType::Error, "Fingerprint timed out");
            assert!(matches!(
                Request::read_from(&mut s).unwrap(),
                Request::PostAuthMessageResponse { response: None }
            ));
            message(&mut s, AuthMessageType::Secret, "Password:");
            assert!(
                matches!(Request::read_from(&mut s).unwrap(),Request::PostAuthMessageResponse {response:Some(value)} if value=="test-secret")
            );
            Response::Success.write_to(&mut s).unwrap();
            assert!(
                matches!(Request::read_from(&mut s).unwrap(),Request::StartSession {cmd,env} if cmd==["niri-session"] && env.contains(&"XDG_SESSION_TYPE=wayland".into()))
            );
            Response::Success.write_to(&mut s).unwrap();
        });
        let client = Client::new(
            path.clone(),
            "niri-session".into(),
            crate::config::Config::default().session_env(),
        )
        .unwrap();
        let start = Instant::now();
        client.start("test-user".into()).unwrap();
        assert!(start.elapsed() < Duration::from_millis(50));
        let events = wait(&client, |event| {
            matches!(event, Event::Info { error: false, .. })
        });
        assert!(events
            .iter()
            .any(|event| matches!(event, Event::Info { .. })));
        wait(&client, |event| {
            matches!(event, Event::Prompt(AuthMessageType::Secret, _))
        });
        client.answer("test-secret".into()).unwrap();
        wait(&client, |event| matches!(event, Event::Started));
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn cancellation_discards_late_fingerprint_success() {
        let (path, listener) = fixture("cancel");
        let server = thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            assert!(matches!(
                Request::read_from(&mut s).unwrap(),
                Request::CreateSession { .. }
            ));
            message(&mut s, AuthMessageType::Info, "Finger auflegen");
            assert!(matches!(
                Request::read_from(&mut s).unwrap(),
                Request::PostAuthMessageResponse { response: None }
            ));
            thread::sleep(Duration::from_millis(150));
            Response::Success.write_to(&mut s).unwrap();
            assert!(
                matches!(Request::read_from(&mut s).unwrap(), Request::CancelSession),
                "cancelled scan must never start session"
            );
            Response::Success.write_to(&mut s).unwrap();
        });
        let client = Client::new(
            path.clone(),
            "niri-session".into(),
            crate::config::Config::default().session_env(),
        )
        .unwrap();
        client.start("test-user".into()).unwrap();
        wait(&client, |event| matches!(event, Event::Info { .. }));
        client.cancel().unwrap();
        let events = wait(&client, |event| matches!(event, Event::Cancelled));
        assert!(!events.iter().any(|event| matches!(event, Event::Started)));
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
