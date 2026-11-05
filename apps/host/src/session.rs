//! One viewer session on the host: authenticate → consent → stream screen / inject input.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use bridge_codec::TileEncoder;
use bridge_crypto::keys::{self, PublicKey};
use bridge_crypto::pake::{self, PakeContext, Role};
use bridge_net::endpoint::{channel_binding, peer_key};
use bridge_platform::{Consent, GamepadInput, InputEvent};
use bridge_proto::features;
use bridge_proto::framing::{read_msg, write_msg, MAX_CLIPBOARD_TEXT, MAX_CONTROL};
use bridge_proto::msg::control::Kind;
use bridge_proto::msg::{
    Bye, ClipboardText, Control, CursorState, FrameUpdate, PakeConfirm, PakeMsg, SessionDenied, SessionGranted,
};
use bridge_session::{FailureOutcome, HostSession, Permissions};
use quinn::{Connection, RecvStream, SendStream};
use tokio::sync::{mpsc, oneshot, Mutex, OwnedSemaphorePermit};
use tracing::{debug, error, info, warn};

use crate::HostCtx;

const AUTH_TIMEOUT: Duration = Duration::from_secs(20);
const CONSENT_TIMEOUT: Duration = Duration::from_secs(60);
const FPS: u64 = 30;
const JPEG_QUALITY: u8 = 65;

type SharedSend = Arc<Mutex<SendStream>>;

async fn send_ctrl(send: &mut SendStream, kind: Kind) -> Result<()> {
    write_msg(send, &Control::new(kind)).await?;
    Ok(())
}

async fn deny(send: &mut SendStream, reason: &str) {
    let _ = send_ctrl(send, Kind::Denied(SessionDenied { reason: reason.into() })).await;
    let _ = send.finish();
    tokio::time::sleep(Duration::from_millis(200)).await; // let it flush before close
}

/// Read the next control message and run it through the state machine.
async fn next(recv: &mut RecvStream, sm: &mut HostSession) -> Result<Kind> {
    let msg: Control = read_msg(recv, MAX_CONTROL).await?;
    let kind = msg.kind.context("empty control message")?;
    sm.admit(&kind)?;
    Ok(kind)
}

struct BusyGuard(Arc<HostCtx>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
    }
}

pub async fn run(ctx: Arc<HostCtx>, conn: Connection, permit: OwnedSemaphorePermit) -> Result<()> {
    let remote = conn.remote_address();
    let viewer_pub = peer_key(&conn).context("viewer presented no Ed25519 key")?;
    let (mut send, mut recv) = tokio::time::timeout(AUTH_TIMEOUT, conn.accept_bi())
        .await
        .context("viewer did not open a control stream")??;
    let mut sm = HostSession::new();

    let auth =
        tokio::time::timeout(AUTH_TIMEOUT, authenticate(&ctx, &conn, &mut sm, &mut send, &mut recv, viewer_pub)).await;
    drop(permit);
    match auth {
        Err(_) => {
            deny(&mut send, "authentication timed out").await;
            bail!("authentication timed out");
        }
        Ok(r) => r?,
    }

    if ctx.busy.swap(true, Ordering::SeqCst) {
        deny(&mut send, "the remote computer is already in a session").await;
        return Ok(());
    }
    let _busy = BusyGuard(ctx.clone());

    let fingerprint = keys::fingerprint(&viewer_pub);
    info!(%remote, %fingerprint, "viewer authenticated, asking local user for consent");
    let fp = fingerprint.clone();
    let addr = remote.to_string();
    let mut dialog = tokio::task::spawn_blocking(move || bridge_platform::ask_consent(&fp, &addr));
    // The dialog closes itself (= reject) if the viewer leaves or nobody answers in time.
    let consent = tokio::select! {
        c = &mut dialog => c?,
        _ = conn.closed() => {
            bridge_platform::close_dialog(bridge_platform::CONSENT_TITLE);
            let _ = dialog.await;
            bail!("viewer left while waiting for consent");
        }
        _ = tokio::time::sleep(CONSENT_TIMEOUT) => {
            bridge_platform::close_dialog(bridge_platform::CONSENT_TITLE);
            let _ = dialog.await;
            deny(&mut send, "the remote user did not answer").await;
            bail!("consent timed out");
        }
    };
    let perms = match consent {
        Consent::FullControl => Permissions::ALL,
        Consent::ViewOnly => Permissions::VIEW,
        Consent::Reject => {
            info!(%remote, "rejected by local user");
            deny(&mut send, "the remote user rejected the connection").await;
            return Ok(());
        }
    };
    sm.grant(perms)?;
    let (width, height) = bridge_platform::screen_size();
    let mut features = features::RELATIVE_MOUSE;
    if perms.contains(Permissions::CONTROL) && tokio::task::spawn_blocking(bridge_platform::gamepad_available).await? {
        features |= features::GAMEPAD;
    }
    send_ctrl(&mut send, Kind::Granted(SessionGranted { permissions: perms.0, width, height, features })).await?;
    info!(%remote, permissions = perms.0, gamepad = features & features::GAMEPAD != 0, "session started");

    active(conn, send, recv, sm, &fingerprint, features).await
}

async fn authenticate(
    ctx: &HostCtx,
    conn: &Connection,
    sm: &mut HostSession,
    send: &mut SendStream,
    recv: &mut RecvStream,
    viewer_pub: PublicKey,
) -> Result<()> {
    let Kind::Hello(_) = next(recv, sm).await? else { bail!("expected Hello") };

    let locked = ctx.limiter.lock().map(|l| l.check(Instant::now())).unwrap_or(Ok(()));
    if let Err(wait) = locked {
        deny(send, &format!("too many failed attempts — try again in {} s", wait.as_secs() + 1)).await;
        bail!("locked out after failed attempts");
    }

    let Kind::PakeMsg(viewer_msg) = next(recv, sm).await? else { bail!("expected PakeMsg") };
    let pctx = PakeContext { channel_binding: channel_binding(conn)?, host_pub: ctx.key.public(), viewer_pub };
    let (state, out) = pake::start(&ctx.code(), &pctx);
    send_ctrl(send, Kind::PakeMsg(PakeMsg { msg: out })).await?;

    let Kind::PakeConfirm(confirm) = next(recv, sm).await? else { bail!("expected PakeConfirm") };
    let secret = state.finish(&viewer_msg.msg).ok().filter(|s| s.verify(Role::Viewer, &confirm.mac));
    let Some(secret) = secret else {
        let outcome = ctx.limiter.lock().ok().map(|mut l| l.record_failure(Instant::now()));
        warn!(remote = %conn.remote_address(), "authentication failed (wrong code or MITM)");
        if let Some(FailureOutcome::RotateSecret { .. }) = outcome {
            warn!("too many failed attempts — code rotated");
            ctx.rotate_code();
        }
        deny(send, "authentication failed — check the code").await;
        bail!("authentication failed");
    };
    if let Ok(mut l) = ctx.limiter.lock() {
        l.record_success();
    }
    ctx.rotate_code(); // single use
    send_ctrl(send, Kind::PakeConfirm(PakeConfirm { mac: secret.confirmation(Role::Host) })).await?;
    Ok(())
}

async fn active(
    conn: Connection,
    send: SendStream,
    mut recv: RecvStream,
    mut sm: HostSession,
    fingerprint: &str,
    features: u32,
) -> Result<()> {
    let perms = sm.permissions();
    let send: SharedSend = Arc::new(Mutex::new(send));
    let stop = Arc::new(AtomicBool::new(false));

    // Video: capture thread → bounded channel (backpressure = natural rate control) → uni stream.
    let mut video = conn.open_uni().await?;
    let (frame_tx, mut frame_rx) = mpsc::channel::<FrameUpdate>(1);
    {
        let stop = stop.clone();
        std::thread::Builder::new().name("capture".into()).spawn(move || capture_loop(frame_tx, stop))?;
    }
    let video_task = tokio::spawn(async move {
        while let Some(frame) = frame_rx.recv().await {
            if write_msg(&mut video, &frame).await.is_err() {
                break;
            }
        }
    });

    // Always-visible indicator with a disconnect button (SR-05).
    let title = format!("Bridge — remote session active [{}]", &fingerprint[..9]);
    let text = format!(
        "Someone is connected to this computer.\n\nViewer key: {fingerprint}\nAccess: {}\n\nClick OK to DISCONNECT immediately.",
        describe(perms)
    );
    let (ind_tx, mut ind_rx) = oneshot::channel::<()>();
    {
        let title = title.clone();
        std::thread::spawn(move || {
            bridge_platform::session_indicator(&title, &text);
            let _ = ind_tx.send(());
        });
    }

    let last_clip = Arc::new(std::sync::Mutex::new(read_clipboard().unwrap_or_default()));
    let clip_task =
        perms.contains(Permissions::CLIPBOARD).then(|| tokio::spawn(clipboard_watch(send.clone(), last_clip.clone())));
    // Lets the viewer switch to relative (pointer-lock) mode when a game / 3D viewport hides the cursor.
    let cursor_task = perms.contains(Permissions::CONTROL).then(|| tokio::spawn(cursor_watch(send.clone())));
    // Virtual pads live on their own thread and are unplugged when `pad_tx` is dropped (session end).
    let pad_tx = (features & features::GAMEPAD != 0).then(|| {
        let (tx, rx) = std::sync::mpsc::channel::<GamepadInput>();
        let _ = std::thread::Builder::new().name("gamepads".into()).spawn(move || gamepad_loop(rx));
        tx
    });

    let reason = loop {
        tokio::select! {
            msg = read_msg::<_, Control>(&mut recv, MAX_CONTROL) => {
                let Ok(msg) = msg else { break "connection closed".to_string() };
                let Some(kind) = msg.kind else { break "malformed message".to_string() };
                if let Err(v) = sm.admit(&kind) {
                    warn!(%v, "protocol violation — closing session");
                    break format!("protocol violation: {v}");
                }
                match kind {
                    Kind::MouseMove(m) => inject(InputEvent::MouseMove { x: m.x, y: m.y }),
                    Kind::MouseMoveRelative(m) => inject(InputEvent::MouseMoveRelative { dx: m.dx, dy: m.dy }),
                    Kind::Gamepad(g) => {
                        if let Some(tx) = &pad_tx {
                            // Values were range-checked by `sm.admit`, so the narrowing casts are lossless.
                            let _ = tx.send(GamepadInput {
                                index: g.index as usize,
                                connected: g.connected,
                                buttons: g.buttons as u16,
                                left_trigger: g.left_trigger as u8,
                                right_trigger: g.right_trigger as u8,
                                left_x: g.left_x as i16,
                                left_y: g.left_y as i16,
                                right_x: g.right_x as i16,
                                right_y: g.right_y as i16,
                            });
                        }
                    }
                    Kind::MouseButton(b) => inject(InputEvent::MouseButton { button: b.button, down: b.down }),
                    Kind::MouseWheel(w) => inject(InputEvent::Wheel { dx: w.dx, dy: w.dy }),
                    Kind::Key(k) => inject(InputEvent::Key { scancode: k.scancode, down: k.down }),
                    Kind::Clipboard(c) => {
                        if let Ok(mut last) = last_clip.lock() {
                            *last = c.text.clone();
                        }
                        let _ = tokio::task::spawn_blocking(move || {
                            arboard::Clipboard::new().and_then(|mut cb| cb.set_text(c.text))
                        }).await;
                    }
                    Kind::Bye(_) => break "viewer disconnected".to_string(),
                    _ => {}
                }
            }
            _ = &mut ind_rx => {
                let mut s = send.lock().await;
                let _ = send_ctrl(&mut s, Kind::Bye(Bye { reason: "disconnected by the remote user".into() })).await;
                break "disconnected by local user".to_string();
            }
            _ = conn.closed() => break "connection lost".to_string(),
        }
    };

    stop.store(true, Ordering::SeqCst);
    video_task.abort();
    for t in [clip_task, cursor_task].into_iter().flatten() {
        t.abort();
    }
    drop(pad_tx); // unplugs virtual gamepads
    sm.close();
    conn.close(0u32.into(), b"bye");
    bridge_platform::close_dialog(&title);
    info!(%reason, "session ended");
    Ok(())
}

/// Report remote cursor visibility changes (polled; cheap Win32 call).
async fn cursor_watch(send: SharedSend) {
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    let mut last: Option<bool> = None;
    loop {
        tick.tick().await;
        let visible = bridge_platform::cursor_visible();
        if last == Some(visible) {
            continue;
        }
        last = Some(visible);
        let mut s = send.lock().await;
        if send_ctrl(&mut s, Kind::CursorState(CursorState { visible })).await.is_err() {
            return;
        }
    }
}

fn gamepad_loop(rx: std::sync::mpsc::Receiver<GamepadInput>) {
    let mut pads = match bridge_platform::VirtualGamepads::new() {
        Ok(p) => p,
        Err(e) => {
            warn!("gamepads unavailable: {e}");
            return;
        }
    };
    info!("virtual gamepad support active");
    while let Ok(input) = rx.recv() {
        if let Err(e) = pads.update(&input) {
            warn!(index = input.index, "gamepad: {e}");
        }
    }
    info!("virtual gamepads unplugged");
}

fn describe(p: Permissions) -> &'static str {
    if p.contains(Permissions::CONTROL) {
        "full control (mouse, keyboard, gamepad, clipboard)"
    } else {
        "view only"
    }
}

fn inject(ev: InputEvent) {
    if let Err(e) = bridge_platform::inject(ev) {
        debug!("input injection failed: {e}");
    }
}

fn read_clipboard() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Send local clipboard changes (made during the session only — never the pre-existing content).
async fn clipboard_watch(send: SharedSend, last: Arc<std::sync::Mutex<String>>) {
    let mut tick = tokio::time::interval(Duration::from_millis(700));
    loop {
        tick.tick().await;
        let Ok(Some(text)) = tokio::task::spawn_blocking(read_clipboard).await else { continue };
        if text.len() > MAX_CLIPBOARD_TEXT {
            continue;
        }
        {
            let Ok(mut l) = last.lock() else { return };
            if *l == text {
                continue;
            }
            *l = text.clone();
        }
        let mut s = send.lock().await;
        if send_ctrl(&mut s, Kind::Clipboard(ClipboardText { text })).await.is_err() {
            return;
        }
    }
}

fn capture_loop(tx: mpsc::Sender<FrameUpdate>, stop: Arc<AtomicBool>) {
    let mut capturer = match bridge_platform::ScreenCapturer::new() {
        Ok(c) => c,
        Err(e) => {
            error!("screen capture unavailable: {e}");
            return;
        }
    };
    let mut encoder = TileEncoder::new(JPEG_QUALITY);
    let mut buf = Vec::new();
    let interval = Duration::from_millis(1000 / FPS);
    while !stop.load(Ordering::Relaxed) {
        let started = Instant::now();
        match capturer.capture(&mut buf) {
            Ok(()) => {
                let (w, h) = capturer.size();
                match encoder.encode(&buf, w, h) {
                    Ok(Some(update)) => {
                        if tx.blocking_send(update).is_err() {
                            break;
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!("encoder: {e}");
                        break;
                    }
                }
            }
            Err(e) => {
                // e.g. secure desktop (UAC/lock screen) is not capturable in portable mode
                debug!("capture failed: {e}");
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        if let Some(rest) = interval.checked_sub(started.elapsed()) {
            std::thread::sleep(rest);
        }
    }
}
