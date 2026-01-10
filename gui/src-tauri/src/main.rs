// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rembrandt_gui::manager::{SessionInfo, SessionManager};
use rembrandt_gui::worktree::{find_repo_root, WorktreeManager};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

/// Application state managed by Tauri
pub struct AppState {
    pub sessions: Mutex<SessionManager>,
    pub worktree_manager: Mutex<Option<WorktreeManager>>,
    pub repo_path: Mutex<Option<PathBuf>>,
}

/// Spawn a new agent
///
/// If `isolated` is true, creates a git worktree for the agent.
/// The agent will work on branch `rembrandt/{agent_id}`.
#[tauri::command]
fn spawn_agent(
    state: State<AppState>,
    agent_id: String,
    command: String,
    workdir: String,
    rows: Option<u16>,
    cols: Option<u16>,
    isolated: Option<bool>,
    base_branch: Option<String>,
) -> Result<String, String> {
    println!("spawn_agent called: agent_id={}, command={}, isolated={:?}", agent_id, command, isolated);

    let args: Vec<&str> = vec![];
    let isolated = isolated.unwrap_or(true); // Default to isolated
    let base_branch = base_branch.unwrap_or_else(|| "main".to_string());

    // First, handle worktree creation without holding session lock
    let (actual_workdir, branch) = if isolated {
        // Find repo root and create worktree
        let workdir_path = PathBuf::from(&workdir);
        let repo_root = find_repo_root(&workdir_path).map_err(|e| {
            println!("Failed to find repo root: {}", e);
            e.to_string()
        })?;

        // Initialize or get WorktreeManager (separate lock scope)
        {
            let mut wt_manager = state.worktree_manager.lock().map_err(|e| e.to_string())?;
            if wt_manager.is_none() {
                *wt_manager = Some(WorktreeManager::new(&repo_root).map_err(|e| {
                    println!("Failed to create WorktreeManager: {}", e);
                    e.to_string()
                })?);
            }
        }

        // Store repo path (separate lock scope)
        {
            let mut repo_path = state.repo_path.lock().map_err(|e| e.to_string())?;
            *repo_path = Some(repo_root);
        }

        // Create the worktree (separate lock scope)
        let worktree_info = {
            let wt_manager = state.worktree_manager.lock().map_err(|e| e.to_string())?;
            wt_manager
                .as_ref()
                .unwrap()
                .create_worktree(&agent_id, &base_branch)
                .map_err(|e| {
                    println!("Failed to create worktree: {}", e);
                    e.to_string()
                })?
        };

        println!("Created worktree at {:?} on branch {}", worktree_info.path, worktree_info.branch);
        (worktree_info.path, Some(worktree_info.branch))
    } else {
        (PathBuf::from(&workdir), None)
    };

    // Now acquire session lock only for spawning
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let session_id = sessions
        .spawn(agent_id.clone(), &command, &args, &actual_workdir, rows, cols, branch.clone(), isolated)
        .map_err(|e| {
            println!("Failed to spawn session: {}", e);
            e.to_string()
        })?;

    println!("Spawned session: {}", session_id);
    Ok(session_id)
}

/// Initialize the worktree manager for a repository
#[tauri::command]
fn init_repo(state: State<AppState>, path: String) -> Result<String, String> {
    let repo_path = PathBuf::from(&path);
    let repo_root = find_repo_root(&repo_path).map_err(|e| e.to_string())?;

    let mut wt_manager = state.worktree_manager.lock().map_err(|e| e.to_string())?;
    *wt_manager = Some(WorktreeManager::new(&repo_root).map_err(|e| e.to_string())?);

    let mut stored_path = state.repo_path.lock().map_err(|e| e.to_string())?;
    *stored_path = Some(repo_root.clone());

    Ok(repo_root.display().to_string())
}

/// Clean up a worktree after an agent is done
#[tauri::command]
fn cleanup_worktree(
    state: State<AppState>,
    agent_id: String,
    delete_branch: Option<bool>,
) -> Result<(), String> {
    let wt_manager = state.worktree_manager.lock().map_err(|e| e.to_string())?;

    if let Some(ref manager) = *wt_manager {
        manager
            .remove_worktree(&agent_id, delete_branch.unwrap_or(false))
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// List all worktrees
#[tauri::command]
fn list_worktrees(state: State<AppState>) -> Result<Vec<rembrandt_gui::worktree::WorktreeInfo>, String> {
    let wt_manager = state.worktree_manager.lock().map_err(|e| e.to_string())?;

    if let Some(ref manager) = *wt_manager {
        manager.list_worktrees().map_err(|e| e.to_string())
    } else {
        Ok(vec![])
    }
}

/// List all agents
#[tauri::command]
fn list_agents(state: State<AppState>) -> Result<Vec<SessionInfo>, String> {
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    // Poll all sessions to update their status (detect exits)
    sessions.poll_all();
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
    println!("write_to_agent: session={} data={:?}", session_id, data);
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    let result = sessions.write(&session_id, &data).map_err(|e| e.to_string());
    println!("write_to_agent: result={:?}", result);
    result
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
    let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    sessions.get_history(&session_id).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            sessions: Mutex::new(SessionManager::new()),
            worktree_manager: Mutex::new(None),
            repo_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            spawn_agent,
            list_agents,
            kill_agent,
            nudge_agent,
            write_to_agent,
            resize_agent,
            get_history,
            init_repo,
            cleanup_worktree,
            list_worktrees,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
