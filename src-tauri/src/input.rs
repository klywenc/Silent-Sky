use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gilrs::{EventType, Gilrs};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mapping {
    pub gamepad: Option<String>,
    pub buttons: HashMap<String, String>,
    pub scratch_axis: Option<String>,
    pub scratch_threshold: f32,
    pub scratch_up_btn: Option<String>,
    pub scratch_down_btn: Option<String>,
}

pub struct InputState {
    pub mapping: Arc<Mutex<Mapping>>,
    pub gamepads: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone, Serialize)]
struct RawInput {
    gamepad: String,
    kind: &'static str,
    code: String,
    value: f32,
}

#[derive(Clone, Serialize)]
struct LaneEvent {
    lane: String,
    active: bool,
}

#[derive(Clone, Serialize)]
struct ScratchEvent {
    dir: &'static str,
    active: bool,
    value: f32,
    source: &'static str,
}

pub fn start(app: AppHandle, state: &InputState) {
    let mapping = state.mapping.clone();
    let gamepads = state.gamepads.clone();

    std::thread::spawn(move || {
        let mut gilrs = match Gilrs::new() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("[input] failed to initialize gilrs: {e:?}");
                return;
            }
        };

        update_gamepads(&gilrs, &app, &gamepads);
        let mut last_axis: HashMap<String, f32> = HashMap::new();
        let mut scratch_acc: HashMap<String, f32> = HashMap::new();

        loop {
            while let Some(ev) = gilrs.next_event() {
                let pad = gilrs
                    .connected_gamepad(ev.id)
                    .map(|g| g.name().to_string())
                    .unwrap_or_default();

                match ev.event {
                    EventType::Connected | EventType::Disconnected => {
                        update_gamepads(&gilrs, &app, &gamepads);
                    }
                    EventType::ButtonPressed(_, code) => {
                        on_button(&app, &mapping, &pad, &code.to_string(), true);
                    }
                    EventType::ButtonReleased(_, code) => {
                        on_button(&app, &mapping, &pad, &code.to_string(), false);
                    }
                    EventType::AxisChanged(_, value, code) => {
                        on_axis(
                            &app,
                            &mapping,
                            &pad,
                            &code.to_string(),
                            value,
                            &mut last_axis,
                            &mut scratch_acc,
                        );
                    }
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });
}

fn update_gamepads(gilrs: &Gilrs, app: &AppHandle, store: &Arc<Mutex<Vec<String>>>) {
    let pads: Vec<String> = gilrs.gamepads().map(|(_, g)| g.name().to_string()).collect();
    *store.lock().unwrap() = pads.clone();
    let _ = app.emit("input://gamepads", pads);
}

fn on_button(app: &AppHandle, mapping: &Arc<Mutex<Mapping>>, pad: &str, code: &str, pressed: bool) {
    let _ = app.emit(
        "input://raw",
        RawInput {
            gamepad: pad.to_string(),
            kind: "button",
            code: code.to_string(),
            value: if pressed { 1.0 } else { 0.0 },
        },
    );

    let m = mapping.lock().unwrap();
    if let Some(g) = &m.gamepad {
        if g != pad {
            return;
        }
    }

    if let Some(lane) = m.buttons.get(code) {
        let _ = app.emit(
            "input://lane",
            LaneEvent {
                lane: lane.clone(),
                active: pressed,
            },
        );
    }

    let is_scratch =
        m.scratch_up_btn.as_deref() == Some(code) || m.scratch_down_btn.as_deref() == Some(code);
    if is_scratch {
        let dir = if m.scratch_up_btn.as_deref() == Some(code) {
            "up"
        } else {
            "down"
        };
        let _ = app.emit(
            "input://scratch",
            ScratchEvent {
                dir: if pressed { dir } else { "none" },
                active: pressed,
                value: if pressed { 1.0 } else { 0.0 },
                source: "button",
            },
        );
    }
}

fn on_axis(
    app: &AppHandle,
    mapping: &Arc<Mutex<Mapping>>,
    pad: &str,
    code: &str,
    value: f32,
    last: &mut HashMap<String, f32>,
    acc: &mut HashMap<String, f32>,
) {
    let _ = app.emit(
        "input://raw",
        RawInput {
            gamepad: pad.to_string(),
            kind: "axis",
            code: code.to_string(),
            value,
        },
    );

    let m = mapping.lock().unwrap();
    if let Some(g) = &m.gamepad {
        if g != pad {
            return;
        }
    }

    if m.scratch_axis.as_deref() == Some(code) {
        let prev = last.get(code).copied().unwrap_or(value);
        last.insert(code.to_string(), value);

        let mut delta = value - prev;
        if delta > 1.0 {
            delta -= 2.0;
        } else if delta < -1.0 {
            delta += 2.0;
        }

        let a = acc.entry(code.to_string()).or_insert(0.0);
        *a += delta;

        let threshold = if m.scratch_threshold > 0.0 {
            m.scratch_threshold
        } else {
            0.012
        };

        if a.abs() >= threshold {
            let dir = if *a > 0.0 { "up" } else { "down" };
            let chunk = *a;
            *a = 0.0;
            let _ = app.emit(
                "input://scratch",
                ScratchEvent {
                    dir,
                    active: true,
                    value: chunk,
                    source: "axis",
                },
            );
        }
    }
}
