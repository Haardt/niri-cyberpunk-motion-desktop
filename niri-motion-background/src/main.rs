use anyhow::Result;
use clap::{Parser, Subcommand};
use motion_render::{app, config, ipc};
use std::path::PathBuf;
#[derive(Parser)]
#[command(
    version,
    about = "Native Wayland GPU shader/video backgrounds with battery-aware frame pacing"
)]
struct Cli {
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Run,
    Check,
    Status,
    List,
    Mode {
        #[arg(value_enum)]
        mode: Mode,
    },
    AutoSwitch {
        #[arg(value_enum)]
        state: SwitchState,
    },
    Next,
    Prev,
    Set {
        name: String,
    },
    Pause,
    Resume,
    Toggle,
    Reload,
    Quit,
}
#[derive(Clone, clap::ValueEnum)]
enum Mode {
    Auto,
    Day,
    Night,
}
#[derive(Clone, clap::ValueEnum)]
enum SwitchState {
    On,
    Off,
    Toggle,
}
fn config_path(explicit: Option<PathBuf>) -> PathBuf {
    explicit.unwrap_or_else(|| {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
            });
        base.join("niri-motion-background/config.toml")
    })
}
fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let cli = Cli::parse();
    match cli.command {
        Command::Run => app::run(config_path(cli.config)),
        Command::Check => {
            let p = config_path(cli.config);
            let c = config::Config::load(&p)?;
            println!(
                "Valid: {} wallpapers; {} FPS / {} battery FPS",
                c.wallpapers.len(),
                c.fps,
                c.battery_fps
            );
            Ok(())
        }
        Command::Mode { mode } => ipc::send(match mode {
            Mode::Auto => "mode auto",
            Mode::Day => "mode day",
            Mode::Night => "mode night",
        }),
        Command::AutoSwitch { state } => ipc::send(match state {
            SwitchState::On => "auto-switch on",
            SwitchState::Off => "auto-switch off",
            SwitchState::Toggle => "auto-switch toggle",
        }),
        Command::Set { name } => {
            anyhow::ensure!(!name.contains(['\n', '\r']), "invalid wallpaper name");
            ipc::send(&format!("set {name}"))
        }
        command => ipc::send(match command {
            Command::Status => "status",
            Command::List => "list",
            Command::Next => "next",
            Command::Prev => "prev",
            Command::Pause => "pause",
            Command::Resume => "resume",
            Command::Toggle => "toggle",
            Command::Reload => "reload",
            Command::Quit => "quit",
            _ => unreachable!(),
        }),
    }
}
