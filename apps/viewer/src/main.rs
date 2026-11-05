//! bridge-viewer — connect to a host by Device ID + one-time code.

mod connect;
mod gamepad;
mod keymap;
mod ui;

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "bridge-viewer", version, about = "Bridge viewer — connect to a remote computer")]
pub struct Args {
    /// Device ID shown on the remote computer (spaces allowed).
    #[arg(num_args = 1.., required = true)]
    pub device_id: Vec<String>,
    /// Rendezvous server `host[:port]` (remembered for next time).
    #[arg(long)]
    pub server: Option<String>,
    /// Rendezvous server key printed by bridge-rendezvous (remembered).
    #[arg(long)]
    pub server_key: Option<String>,
    /// Configuration directory (default: %APPDATA%\Bridge).
    #[arg(long)]
    pub config_dir: Option<PathBuf>,
    /// Accept a CHANGED host key. Only after verifying the new fingerprint with the remote user!
    #[arg(long)]
    pub accept_new_host_key: bool,
    /// Do not switch to relative mouse mode automatically when the remote cursor is hidden
    /// (Ctrl+Alt+M still toggles it manually).
    #[arg(long)]
    pub no_auto_relative: bool,
    /// Do not forward local gamepads to the remote computer.
    #[arg(long)]
    pub no_gamepad: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,quinn=warn".into()),
        )
        .init();
    let args = Args::parse();
    let device_id: String = args.device_id.concat().chars().filter(char::is_ascii_digit).collect();

    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let session = match rt.block_on(connect::connect(&args, &device_id)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("\nConnection failed: {e:#}");
            std::process::exit(1);
        }
    };
    let opts = ui::UiOptions { auto_relative: !args.no_auto_relative, gamepad: !args.no_gamepad };
    let result = ui::run(&rt, session, &device_id, opts);
    rt.shutdown_timeout(std::time::Duration::from_secs(1));
    result
}
