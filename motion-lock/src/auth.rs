//! PAM runs off the Wayland/render thread. There is no preview authentication path.
use anyhow::{bail, Context as _, Result};
use pam_client::{Context, ConversationHandler, ErrorCode, Flag};
use std::{
    ffi::{CStr, CString},
    path::Path,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};
use zeroize::Zeroizing;

pub enum Event {
    Prompt(String, bool),
    Notice(String),
    Result(bool),
}
pub struct Auth {
    pub events: Receiver<Event>,
    begin: SyncSender<Zeroizing<String>>,
    answers: SyncSender<Zeroizing<String>>,
}
struct Conversation {
    first: Option<Zeroizing<String>>,
    events: mpsc::Sender<Event>,
    answers: Receiver<Zeroizing<String>>,
}
impl Conversation {
    fn prompt(&mut self, msg: &CStr, secret: bool) -> Result<CString, ErrorCode> {
        let value = if secret && self.first.is_some() {
            self.first.take().unwrap()
        } else {
            self.events
                .send(Event::Prompt(msg.to_string_lossy().into_owned(), secret))
                .map_err(|_| ErrorCode::ABORT)?;
            self.answers.recv().map_err(|_| ErrorCode::ABORT)?
        };
        CString::new(value.as_str()).map_err(|_| ErrorCode::ABORT)
    }
}
impl ConversationHandler for Conversation {
    fn init(&mut self, _: Option<impl AsRef<str>>) {}
    fn prompt_echo_on(&mut self, msg: &CStr) -> Result<CString, ErrorCode> {
        self.prompt(msg, false)
    }
    fn prompt_echo_off(&mut self, msg: &CStr) -> Result<CString, ErrorCode> {
        self.prompt(msg, true)
    }
    fn text_info(&mut self, msg: &CStr) {
        let _ = self
            .events
            .send(Event::Notice(msg.to_string_lossy().into_owned()));
    }
    fn error_msg(&mut self, msg: &CStr) {
        self.text_info(msg);
    }
    fn radio_prompt(&mut self, _: &CStr) -> Result<bool, ErrorCode> {
        Err(ErrorCode::ABORT)
    }
}
impl Auth {
    pub fn new(service: &str) -> Result<Self> {
        if service.is_empty()
            || service.contains('/')
            || !Path::new("/etc/pam.d").join(service).is_file()
        {
            bail!("PAM service /etc/pam.d/{service} is missing; install packaging/motion-lock.pam before locking");
        }
        let (begin, starts) = mpsc::sync_channel::<Zeroizing<String>>(1);
        let (answers, responses) = mpsc::sync_channel(1);
        let (events, rx) = mpsc::channel();
        let (ready, initialized) = mpsc::sync_channel(1);
        let service = service.to_owned();
        thread::Builder::new()
            .name("motion-lock-pam".into())
            .spawn(move || {
                let conversation = Conversation {
                    first: None,
                    events: events.clone(),
                    answers: responses,
                };
                let username = whoami::username();
                let mut context = match Context::new(&service, Some(&username), conversation) {
                    Ok(c) => {
                        let _ = ready.send(Ok(()));
                        c
                    }
                    Err(e) => {
                        let _ = ready.send(Err(format!("PAM initialization failed: {e}")));
                        return;
                    }
                };
                while let Ok(password) = starts.recv() {
                    context.conversation_mut().first = Some(password);
                    // Both credentials AND the system's account policy must allow unlock.
                    let success = context
                        .authenticate(Flag::NONE)
                        .and_then(|_| context.acct_mgmt(Flag::NONE))
                        .is_ok();
                    context.conversation_mut().first = None;
                    if events.send(Event::Result(success)).is_err() {
                        break;
                    }
                }
            })?;
        initialized
            .recv()
            .context("PAM worker stopped")?
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            events: rx,
            begin,
            answers,
        })
    }
    pub fn submit(&self, value: Zeroizing<String>, continuation: bool) -> Result<()> {
        let tx = if continuation {
            &self.answers
        } else {
            &self.begin
        };
        tx.try_send(value)
            .map_err(|_| anyhow::anyhow!("PAM worker unavailable or busy"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    #[test]
    fn missing_pam_service_refuses_before_locking() {
        assert!(Auth::new("motion-lock-missing-service-test").is_err());
        assert!(Auth::new("../system-auth").is_err());
    }
    #[test]
    fn real_pam_deny_never_returns_success() {
        // Uses Arch's explicit deny-all fallback, never the user's real password.
        if !Path::new("/etc/pam.d/other").exists() {
            return;
        }
        let a = Auth::new("other").unwrap();
        a.submit(
            Zeroizing::new("motion-lock-noncredential-test".into()),
            false,
        )
        .unwrap();
        let end = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(Instant::now() < end, "PAM denial timed out");
            match a.events.recv_timeout(Duration::from_secs(1)) {
                Ok(Event::Result(result)) => {
                    assert!(!result);
                    break;
                }
                Ok(Event::Prompt(_, _)) => {
                    a.submit(Zeroizing::new(String::new()), true).unwrap();
                }
                Ok(Event::Notice(_)) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(e) => panic!("PAM worker disconnected: {e}"),
            }
        }
    }
}

// Fingerprint PAM is isolated in a child process so password authentication can
// run concurrently and cancelling/killing the scanner releases its D-Bus claim.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum FingerprintEvent {
    Notice { message: String },
    Complete { success: bool },
}
fn biometric_emit(event: FingerprintEvent) {
    use std::io::Write;
    if let Ok(line) = serde_json::to_string(&event) {
        println!("{line}");
        let _ = std::io::stdout().flush();
    }
}
struct BiometricConversation;
impl ConversationHandler for BiometricConversation {
    fn init(&mut self, _: Option<impl AsRef<str>>) {}
    fn prompt_echo_on(&mut self, _: &CStr) -> Result<CString, ErrorCode> {
        Err(ErrorCode::ABORT)
    }
    fn prompt_echo_off(&mut self, _: &CStr) -> Result<CString, ErrorCode> {
        Err(ErrorCode::ABORT)
    }
    fn text_info(&mut self, msg: &CStr) {
        biometric_emit(FingerprintEvent::Notice {
            message: msg.to_string_lossy().into_owned(),
        });
    }
    fn error_msg(&mut self, msg: &CStr) {
        self.text_info(msg);
    }
    fn radio_prompt(&mut self, _: &CStr) -> Result<bool, ErrorCode> {
        Err(ErrorCode::ABORT)
    }
}
pub fn fingerprint_worker(service: &str) -> Result<()> {
    if service.is_empty()
        || service.contains('/')
        || !Path::new("/etc/pam.d").join(service).is_file()
    {
        bail!("fingerprint PAM service is not installed");
    }
    let limit = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    unsafe {
        libc::setrlimit(libc::RLIMIT_CORE, &limit);
        libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0);
    }
    let username = whoami::username();
    let mut context = Context::new(service, Some(&username), BiometricConversation)?;
    let success = context
        .authenticate(Flag::NONE)
        .and_then(|_| context.acct_mgmt(Flag::NONE))
        .is_ok();
    biometric_emit(FingerprintEvent::Complete { success });
    Ok(())
}
pub struct Fingerprint {
    child: std::process::Child,
    pub events: Receiver<FingerprintEvent>,
}
impl Fingerprint {
    pub fn start(service: &str) -> Result<Self> {
        use std::{
            io::{BufRead, BufReader, Read},
            os::unix::process::CommandExt,
            process::{Command, Stdio},
        };
        if service.is_empty()
            || service.contains('/')
            || !Path::new("/etc/pam.d").join(service).is_file()
        {
            bail!("fingerprint PAM service is not installed");
        }
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args(["--fingerprint-worker", "--pam-service", service])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let parent = unsafe { libc::getpid() };
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    libc::_exit(1);
                }
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        let pipe = child
            .stdout
            .take()
            .context("fingerprint output unavailable")?;
        let (tx, events) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(pipe.take(32768)).lines().take(64) {
                let Ok(line) = line else {
                    break;
                };
                let Ok(event) = serde_json::from_str::<FingerprintEvent>(&line) else {
                    break;
                };
                if tx.send(event).is_err() {
                    break;
                }
            }
        });
        Ok(Self { child, events })
    }
}
impl Drop for Fingerprint {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
