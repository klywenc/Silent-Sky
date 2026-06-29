use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

#[cfg(windows)]
use windows::Win32::Foundation::{HWND, RECT};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowRect, GetWindowTextW};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachConfig {
    pub enabled: bool,
    pub target_title: String,
    pub anchor: String,
    pub margin: i32,
    pub hide_when_inactive: bool,
}

impl Default for AttachConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_title: "LR2oraja".into(),
            anchor: "top-right".into(),
            margin: 16,
            hide_when_inactive: true,
        }
    }
}

pub struct AttachState {
    pub cfg: Arc<Mutex<AttachConfig>>,
}

#[derive(Clone, Serialize)]
struct AttachStatus {
    active: bool,
    title: String,
}

#[cfg(windows)]
pub fn start(app: AppHandle, state: &AttachState) {
    let cfg = state.cfg.clone();

    std::thread::spawn(move || {
        let window = app.get_webview_window("main");
        let our_hwnd: isize = window
            .as_ref()
            .and_then(|w| w.hwnd().ok())
            .map(|h| h.0 as isize)
            .unwrap_or(0);

        loop {
            let c = cfg.lock().unwrap().clone();
            if c.enabled {
                if let Some(w) = &window {
                    tick(&app, w, our_hwnd, &c);
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    });
}

#[cfg(not(windows))]
pub fn start(_app: AppHandle, _state: &AttachState) {}

#[cfg(windows)]
fn tick(app: &AppHandle, w: &WebviewWindow, our_hwnd: isize, c: &AttachConfig) {
    let fg = unsafe { GetForegroundWindow() };
    let fg_id = fg.0 as isize;
    let title = window_title(fg);

    let is_game = !c.target_title.is_empty()
        && title.to_lowercase().contains(&c.target_title.to_lowercase());

    if is_game {
        if let Some(rect) = window_rect(fg) {
            reposition(w, &rect, c);
        }
        let _ = w.show();
        let _ = app.emit("attach://status", AttachStatus { active: true, title });
    } else if fg_id == our_hwnd {
    } else if c.hide_when_inactive {
        let _ = w.hide();
        let _ = app.emit(
            "attach://status",
            AttachStatus { active: false, title: String::new() },
        );
    }
}

#[cfg(windows)]
fn window_title(hwnd: HWND) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if len <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..len as usize])
}

#[cfg(windows)]
fn window_rect(hwnd: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    let ok = unsafe { GetWindowRect(hwnd, &mut r) };
    ok.is_ok().then_some(r)
}

#[cfg(windows)]
fn reposition(w: &WebviewWindow, game: &RECT, c: &AttachConfig) {
    let (ww, wh) = w
        .outer_size()
        .map(|s| (s.width as i32, s.height as i32))
        .unwrap_or((560, 380));
    let m = c.margin;
    let (x, y) = match c.anchor.as_str() {
        "top-left" => (game.left + m, game.top + m),
        "bottom-left" => (game.left + m, game.bottom - wh - m),
        "bottom-right" => (game.right - ww - m, game.bottom - wh - m),
        _ => (game.right - ww - m, game.top + m),
    };
    let _ = w.set_position(PhysicalPosition::new(x, y));
}
