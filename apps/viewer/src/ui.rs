//! Viewer window: winit + softbuffer (CPU blit, fit-to-window). GPU rendering comes with HW decode.

use std::collections::HashSet;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use bridge_codec::FrameDecoder;
use bridge_proto::button;
use bridge_proto::framing::{read_msg, write_msg, MAX_CLIPBOARD_TEXT, MAX_CONTROL, MAX_FRAME};
use bridge_proto::msg::control::Kind;
use bridge_proto::msg::{
    Bye, ClipboardText, Control, FrameUpdate, KeyEvent, MouseButton as MouseButtonMsg, MouseMove, MouseWheel,
};
use bridge_session::Permissions;
use tokio::sync::mpsc;
use tracing::warn;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::platform::scancode::PhysicalKeyExtScancode;
use winit::window::{Window, WindowId};

use crate::connect::Session;

#[derive(Debug)]
enum UserEvent {
    Frame,
    Closed(String),
}

#[derive(Default)]
struct FrameSlot {
    width: usize,
    height: usize,
    pixels: Vec<u32>,
}

struct Shared {
    frame: Mutex<FrameSlot>,
    pending: AtomicBool,
}

#[derive(Clone, Copy, Default)]
struct Viewport {
    ox: usize,
    oy: usize,
    w: usize,
    h: usize,
    src_w: usize,
    src_h: usize,
}

impl Viewport {
    fn to_remote(self, x: f64, y: f64) -> Option<(i32, i32)> {
        if self.w == 0 || self.h == 0 {
            return None;
        }
        let rx = ((x - self.ox as f64) * self.src_w as f64 / self.w as f64).floor();
        let ry = ((y - self.oy as f64) * self.src_h as f64 / self.h as f64).floor();
        if rx < 0.0 || ry < 0.0 || rx >= self.src_w as f64 || ry >= self.src_h as f64 {
            return None;
        }
        Some((rx as i32, ry as i32))
    }
}

pub fn run(rt: &tokio::runtime::Runtime, session: Session, device_id: &str) -> Result<()> {
    let perms = Permissions::sanitize(session.granted.permissions);
    let control = perms.contains(Permissions::CONTROL);
    println!(
        "Connected ({}). Access: {}. Close the window to disconnect.",
        session.path,
        if control { "full control" } else { "view only" }
    );

    let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let shared = Arc::new(Shared { frame: Mutex::new(FrameSlot::default()), pending: AtomicBool::new(false) });
    let (input_tx, input_rx) = mpsc::channel::<Kind>(1024);
    let last_clip = Arc::new(Mutex::new(String::new()));

    let Session { conn, send, recv, granted: _, _relay, _endpoint, path: _ } = session;
    let _guard = rt.enter();
    rt.spawn(video_task(conn.clone(), shared.clone(), proxy.clone()));
    rt.spawn(writer_task(send, input_rx));
    rt.spawn(reader_task(recv, last_clip.clone(), proxy.clone()));
    if perms.contains(Permissions::CLIPBOARD) {
        *last_clip.lock().unwrap_or_else(|e| e.into_inner()) = read_clipboard().unwrap_or_default();
        rt.spawn(clipboard_task(input_tx.clone(), last_clip.clone()));
    }

    let formatted: String =
        device_id.as_bytes().chunks(3).map(|c| String::from_utf8_lossy(c).into_owned()).collect::<Vec<_>>().join(" ");
    let mut app = App {
        title: format!("Bridge — {formatted} — {}", if control { "full control" } else { "view only" }),
        window: None,
        surface: None,
        context: None,
        shared,
        view: Viewport::default(),
        input: input_tx,
        control,
        pressed: HashSet::new(),
        last_pos: None,
        closed: None,
    };
    event_loop.run_app(&mut app)?;
    conn.close(0u32.into(), b"bye");
    // keep relay/endpoint alive until here
    drop((_relay, _endpoint));
    if let Some(reason) = app.closed {
        println!("Session ended: {reason}");
    }
    Ok(())
}

async fn video_task(conn: quinn::Connection, shared: Arc<Shared>, proxy: EventLoopProxy<UserEvent>) {
    let reason = async {
        let mut video = conn.accept_uni().await.map_err(|e| e.to_string())?;
        let mut decoder = FrameDecoder::new();
        loop {
            let update: FrameUpdate = read_msg(&mut video, MAX_FRAME).await.map_err(|e| e.to_string())?;
            // SEC: M-53 — reject malformed or out-of-bounds updates from the host.
            decoder.apply(&update).map_err(|e| format!("invalid video data from host: {e}"))?;
            {
                let mut slot = shared.frame.lock().unwrap_or_else(|e| e.into_inner());
                slot.width = decoder.width() as usize;
                slot.height = decoder.height() as usize;
                slot.pixels.clear();
                slot.pixels.extend_from_slice(decoder.pixels());
            }
            if !shared.pending.swap(true, Ordering::AcqRel) {
                let _ = proxy.send_event(UserEvent::Frame);
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), String>(())
    }
    .await;
    if let Err(r) = reason {
        let _ = proxy.send_event(UserEvent::Closed(r));
    }
}

async fn writer_task(mut send: quinn::SendStream, mut rx: mpsc::Receiver<Kind>) {
    while let Some(kind) = rx.recv().await {
        let bye = matches!(kind, Kind::Bye(_));
        if write_msg(&mut send, &Control::new(kind)).await.is_err() || bye {
            break;
        }
    }
    let _ = send.finish();
}

async fn reader_task(mut recv: quinn::RecvStream, last_clip: Arc<Mutex<String>>, proxy: EventLoopProxy<UserEvent>) {
    let reason = loop {
        let msg: Control = match read_msg(&mut recv, MAX_CONTROL).await {
            Ok(m) => m,
            Err(_) => break "connection closed".to_string(),
        };
        match msg.kind {
            Some(Kind::Clipboard(c)) if c.text.len() <= MAX_CLIPBOARD_TEXT => {
                *last_clip.lock().unwrap_or_else(|e| e.into_inner()) = c.text.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    arboard::Clipboard::new().and_then(|mut cb| cb.set_text(c.text))
                })
                .await;
            }
            Some(Kind::Bye(b)) => break b.reason,
            Some(other) => warn!("ignoring unexpected {} from host", other.name()),
            None => {}
        }
    };
    let _ = proxy.send_event(UserEvent::Closed(reason));
}

fn read_clipboard() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

async fn clipboard_task(tx: mpsc::Sender<Kind>, last: Arc<Mutex<String>>) {
    let mut tick = tokio::time::interval(Duration::from_millis(700));
    loop {
        tick.tick().await;
        let Ok(Some(text)) = tokio::task::spawn_blocking(read_clipboard).await else { continue };
        if text.len() > MAX_CLIPBOARD_TEXT {
            continue;
        }
        {
            let mut l = last.lock().unwrap_or_else(|e| e.into_inner());
            if *l == text {
                continue;
            }
            *l = text.clone();
        }
        if tx.send(Kind::Clipboard(ClipboardText { text })).await.is_err() {
            return;
        }
    }
}

struct App {
    title: String,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    context: Option<softbuffer::Context<Rc<Window>>>,
    shared: Arc<Shared>,
    view: Viewport,
    input: mpsc::Sender<Kind>,
    control: bool,
    pressed: HashSet<u32>,
    last_pos: Option<(i32, i32)>,
    closed: Option<String>,
}

impl App {
    fn send(&self, kind: Kind) {
        // Never block the UI thread; drop events if the network is congested.
        let _ = self.input.try_send(kind);
    }

    fn release_all_keys(&mut self) {
        for sc in std::mem::take(&mut self.pressed) {
            self.send(Kind::Key(KeyEvent { scancode: sc, down: false }));
        }
    }

    fn redraw(&mut self) {
        self.shared.pending.store(false, Ordering::Release);
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else { return };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else { return };
        if surface.resize(w, h).is_err() {
            return;
        }
        let Ok(mut buffer) = surface.buffer_mut() else { return };
        let frame = self.shared.frame.lock().unwrap_or_else(|e| e.into_inner());
        self.view = blit(&mut buffer, w.get() as usize, h.get() as usize, &frame);
        drop(frame);
        let _ = buffer.present();
    }
}

/// Nearest-neighbour fit-to-window with letterboxing.
fn blit(dst: &mut [u32], dw: usize, dh: usize, src: &FrameSlot) -> Viewport {
    dst.fill(0x0020_2020);
    let (sw, sh) = (src.width, src.height);
    if sw == 0 || sh == 0 || src.pixels.len() < sw * sh {
        return Viewport::default();
    }
    let scale = (dw as f64 / sw as f64).min(dh as f64 / sh as f64);
    let tw = ((sw as f64 * scale) as usize).clamp(1, dw);
    let th = ((sh as f64 * scale) as usize).clamp(1, dh);
    let ox = (dw - tw) / 2;
    let oy = (dh - th) / 2;
    let xmap: Vec<usize> = (0..tw).map(|x| x * sw / tw).collect();
    for y in 0..th {
        let sy = y * sh / th;
        let src_row = &src.pixels[sy * sw..(sy + 1) * sw];
        let dst_row = &mut dst[(oy + y) * dw + ox..][..tw];
        for (d, &sx) in dst_row.iter_mut().zip(&xmap) {
            *d = src_row[sx];
        }
    }
    Viewport { ox, oy, w: tw, h: th, src_w: sw, src_h: sh }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs =
            Window::default_attributes().with_title(&self.title).with_inner_size(LogicalSize::new(1280.0, 760.0));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Rc::new(w),
            Err(e) => {
                self.closed = Some(format!("cannot create window: {e}"));
                event_loop.exit();
                return;
            }
        };
        let Ok(context) = softbuffer::Context::new(window.clone()) else {
            event_loop.exit();
            return;
        };
        let Ok(surface) = softbuffer::Surface::new(&context, window.clone()) else {
            event_loop.exit();
            return;
        };
        self.context = Some(context);
        self.surface = Some(surface);
        self.window = Some(window);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Frame => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            UserEvent::Closed(reason) => {
                self.closed = Some(reason);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.release_all_keys();
                self.send(Kind::Bye(Bye { reason: "viewer closed the window".into() }));
                self.closed.get_or_insert_with(|| "closed by you".into());
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::Resized(_) => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::Focused(false) => self.release_all_keys(),
            _ if !self.control => {}
            WindowEvent::CursorMoved { position, .. } => {
                if let Some(p) = self.view.to_remote(position.x, position.y) {
                    if self.last_pos != Some(p) {
                        self.last_pos = Some(p);
                        self.send(Kind::MouseMove(MouseMove { x: p.0, y: p.1 }));
                    }
                }
            }
            WindowEvent::MouseInput { state, button: b, .. } => {
                let button = match b {
                    MouseButton::Left => button::LEFT,
                    MouseButton::Right => button::RIGHT,
                    MouseButton::Middle => button::MIDDLE,
                    _ => return,
                };
                self.send(Kind::MouseButton(MouseButtonMsg { button, down: state == ElementState::Pressed }));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => ((x * 120.0) as i32, (y * 120.0) as i32),
                    MouseScrollDelta::PixelDelta(p) => (p.x as i32, p.y as i32),
                };
                if dx != 0 || dy != 0 {
                    self.send(Kind::MouseWheel(MouseWheel { dx, dy }));
                }
            }
            WindowEvent::KeyboardInput { event, is_synthetic: false, .. } => {
                let Some(sc) = event.physical_key.to_scancode() else { return };
                if sc == 0 {
                    return;
                }
                let down = event.state == ElementState::Pressed;
                if down {
                    self.pressed.insert(sc);
                } else {
                    self.pressed.remove(&sc);
                }
                self.send(Kind::Key(KeyEvent { scancode: sc, down }));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letterbox_and_mapping() {
        let src = FrameSlot { width: 200, height: 100, pixels: vec![0xFF0000; 200 * 100] };
        let mut dst = vec![0u32; 400 * 400];
        let v = blit(&mut dst, 400, 400, &src);
        assert_eq!((v.ox, v.oy, v.w, v.h), (0, 100, 400, 200));
        assert_eq!(dst[0], 0x0020_2020, "letterbox");
        assert_eq!(dst[150 * 400 + 10], 0xFF0000);
        assert_eq!(v.to_remote(0.0, 50.0), None);
        assert_eq!(v.to_remote(399.0, 299.0), Some((199, 99)));
        assert_eq!(v.to_remote(200.0, 200.0), Some((100, 50)));
    }
}
