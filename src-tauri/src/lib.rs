mod attach;
mod db;
mod input;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use attach::{AttachConfig, AttachState};
use db::{DbConfig, DbState, ScoreRow};
use input::{InputState, Mapping};
use tauri::{AppHandle, Manager, State};

fn config_file(app: &AppHandle, name: &str) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join(name))
}

fn load_json<T: serde::de::DeserializeOwned + Default>(app: &AppHandle, name: &str) -> T {
    if let Some(p) = config_file(app, name) {
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<T>(&s) {
                return v;
            }
        }
    }
    T::default()
}

fn save_json<T: serde::Serialize>(app: &AppHandle, name: &str, value: &T) -> std::io::Result<()> {
    if let Some(p) = config_file(app, name) {
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into());
        std::fs::write(p, json)?;
    }
    Ok(())
}

#[tauri::command]
fn get_mapping(state: State<InputState>) -> Mapping {
    state.mapping.lock().unwrap().clone()
}

#[tauri::command]
fn set_mapping(app: AppHandle, state: State<InputState>, mapping: Mapping) -> Result<(), String> {
    *state.mapping.lock().unwrap() = mapping.clone();
    save_json(&app, "mapping.json", &mapping).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_gamepads(state: State<InputState>) -> Vec<String> {
    state.gamepads.lock().unwrap().clone()
}

#[tauri::command]
fn get_attach_config(state: State<AttachState>) -> AttachConfig {
    state.cfg.lock().unwrap().clone()
}

#[tauri::command]
fn set_attach_config(
    app: AppHandle,
    state: State<AttachState>,
    config: AttachConfig,
) -> Result<(), String> {
    *state.cfg.lock().unwrap() = config.clone();
    save_json(&app, "attach.json", &config).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_db_config(state: State<DbState>) -> DbConfig {
    state.cfg.lock().unwrap().clone()
}

#[tauri::command]
fn set_db_root(app: AppHandle, state: State<DbState>, root: String) -> Result<(), String> {
    let cfg = DbConfig {
        root: if root.trim().is_empty() {
            None
        } else {
            Some(root.trim().to_string())
        },
    };
    *state.cfg.lock().unwrap() = cfg.clone();
    db::invalidate_tables(&state);
    save_json(&app, "db.json", &cfg).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_recent_scores(state: State<DbState>, limit: i64) -> Result<Vec<ScoreRow>, String> {
    let root = state
        .cfg
        .lock()
        .unwrap()
        .root
        .clone()
        .ok_or_else(|| "Database folder is not set.".to_string())?;
    let tables = db::get_or_build_tables(&state, &root);
    db::recent_scores(&root, limit, &tables)
}

#[tauri::command]
fn rescan_tables(state: State<DbState>) {
    db::invalidate_tables(&state);
}

#[tauri::command]
fn db_list_tables(path: String) -> Result<Vec<String>, String> {
    db::list_tables(&path)
}

#[tauri::command]
fn db_diagnostics(state: State<DbState>) -> Result<String, String> {
    let root = state
        .cfg
        .lock()
        .unwrap()
        .root
        .clone()
        .ok_or_else(|| "Set the database folder first.".to_string())?;
    db::diagnostics(&root)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();

            let input_state = InputState {
                mapping: Arc::new(Mutex::new(load_json::<Mapping>(&handle, "mapping.json"))),
                gamepads: Arc::new(Mutex::new(Vec::new())),
            };
            input::start(handle.clone(), &input_state);
            app.manage(input_state);

            let attach_state = AttachState {
                cfg: Arc::new(Mutex::new(load_json::<AttachConfig>(&handle, "attach.json"))),
            };
            attach::start(handle.clone(), &attach_state);
            app.manage(attach_state);

            let db_state = DbState {
                cfg: Arc::new(Mutex::new(load_json::<DbConfig>(&handle, "db.json"))),
                tables: Arc::new(Mutex::new(None)),
            };
            app.manage(db_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_mapping,
            set_mapping,
            get_gamepads,
            get_attach_config,
            set_attach_config,
            get_db_config,
            set_db_root,
            get_recent_scores,
            rescan_tables,
            db_list_tables,
            db_diagnostics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
