// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rembrandt_gui::manager::{SessionInfo, SessionManager};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

/// Application state managed by Tauri
pub struct AppState {
    pub sessions: Mutex<SessionManager>,
}

/// Spawn a new agent
#[tauri::command]
fn spawn_agent(
    state: State<AppState>,
    agent_id: String,
    command: String,
    workdir: String,
    rows: Option<u16>,
    cols: Option<u16>,
) -> Result<String, String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let args: Vec<&str> = vec![];
    let path = PathBuf::from(&workdir);

    sessions
        .spawn(agent_id, &command, &args, &path, rows, cols)
        .map_err(|e| e.to_string())
}

/// List all agents
#[tauri::command]
fn list_agents(state: State<AppState>) -> Result<Vec<SessionInfo>, String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    Ok(sessions.list())
}

/// Kill an agent
#[tauri::command]
fn kill_agent(state: State<AppState>, session_id: String) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    sessions.kill(&session_id).map_err(|e| e.to_string())
}

/// Nudge an agent
#[tauri::command]
fn nudge_agent(state: State<AppState>, session_id: String) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    sessions.nudge(&session_id).map_err(|e| e.to_string())
}

/// Write to an agent's PTY
#[tauri::command]
fn write_to_agent(
    state: State<AppState>,
    session_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    sessions.write(&session_id, &data).map_err(|e| e.to_string())
}

/// Resize an agent's PTY
#[tauri::command]
fn resize_agent(
    state: State<AppState>,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    sessions
        .resize(&session_id, rows, cols)
        .map_err(|e| e.to_string())
}

/// Get output history for an agent
#[tauri::command]
fn get_history(state: State<AppState>, session_id: String) -> Result<Vec<u8>, String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    sessions.get_history(&session_id).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            sessions: Mutex::new(SessionManager::new()),
        })
        .invoke_handler(tauri::generate_handler![
            spawn_agent,
            list_agents,
            kill_agent,
            nudge_agent,
            write_to_agent,
            resize_agent,
            get_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
