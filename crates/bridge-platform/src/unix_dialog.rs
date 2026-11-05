//! Native dialogs on macOS and Linux via helper processes (osascript / zenity / kdialog).
//!
//! SEC: all text is passed as separate argv entries, never through a shell or interpolated into a
//! script, so the remote side cannot inject commands via fingerprints or addresses.
//! Dialogs can be closed programmatically (consent timeout, session end) by killing the helper;
//! a killed consent dialog counts as **reject**.

use std::collections::HashMap;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

type Shared = Arc<Mutex<Child>>;

fn registry() -> &'static Mutex<HashMap<String, Shared>> {
    static REG: OnceLock<Mutex<HashMap<String, Shared>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Run a dialog helper and wait for it. `None` if it could not start or was killed.
pub(crate) fn run(title: &str, mut cmd: Command) -> Option<(i32, String)> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = cmd.spawn().ok()?;
    let mut stdout = child.stdout.take();
    let child: Shared = Arc::new(Mutex::new(child));
    if let Ok(mut r) = registry().lock() {
        r.insert(title.to_string(), child.clone());
    }
    // Poll instead of a blocking wait so `close_dialog` can lock the child and kill it.
    let status = loop {
        let polled = child.lock().ok().and_then(|mut c| c.try_wait().transpose());
        match polled {
            Some(Ok(status)) => break Some(status),
            Some(Err(_)) => break None,
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    if let Ok(mut r) = registry().lock() {
        r.remove(title);
    }
    let code = status?.code()?; // None when killed by a signal
    let mut text = String::new();
    if let Some(out) = stdout.as_mut() {
        let _ = out.read_to_string(&mut text);
    }
    Some((code, text.trim().to_string()))
}

/// Is `tool` installed (and runnable)? (Linux chooses between zenity and kdialog.)
#[cfg(target_os = "linux")]
pub(crate) fn have(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Close a dialog started by [`run`] with this title.
pub fn close_dialog(title: &str) {
    let child = registry().lock().ok().and_then(|r| r.get(title).cloned());
    if let Some(child) = child {
        if let Ok(mut c) = child.lock() {
            let _ = c.kill(); // safe even if it already exited: std tracks the reaped status
        }
    }
}
