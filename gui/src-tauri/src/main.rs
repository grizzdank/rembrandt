// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use rembrandt_gui::beads::{self, BeadsTask};
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
/// If `task_id` is provided, claims the task in Beads.
/// If `initial_prompt` is provided, passes it to Claude via -p flag.
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
    task_id: Option<String>,
    initial_prompt: Option<String>,
) -> Result<String, String> {
    println!("spawn_agent called: agent_id={}, command={}, isolated={:?}, task_id={:?}, has_prompt={}",
             agent_id, command, isolated, task_id, initial_prompt.is_some());

    // Build args based on command type
    // - claude: uses -p "prompt"
    // - opencode: uses --prompt "prompt"
    let prompt_owned: String;
    let args: Vec<&str> = if let Some(ref prompt) = initial_prompt {
        prompt_owned = prompt.clone();
        if command.contains("opencode") {
            vec!["--prompt", &prompt_owned]
        } else {
            // Default to claude-style -p flag
            vec!["-p", &prompt_owned]
        }
    } else {
        vec![]
    };
    let isolated = isolated.unwrap_or(true); // Default to isolated
    let base_branch = base_branch.unwrap_or_else(|| "main".to_string());

    // If task_id provided, fetch task info and claim it
    let (task_id, task_title) = if let Some(tid) = task_id {
        let title = beads::get_task(&tid)
            .ok()
            .flatten()
            .map(|t| t.title);

        // Claim the task (set to in_progress)
        if let Err(e) = beads::claim_task(&tid, &agent_id) {
            println!("Warning: Failed to claim task {}: {}", tid, e);
        }

        (Some(tid), title)
    } else {
        (None, None)
    };

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
        .spawn(agent_id.clone(), &command, &args, &actual_workdir, rows, cols, branch.clone(), isolated, task_id, task_title)
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
    let newly_exited = sessions.poll_all();

    // Update Beads task status for agents that exited with errors
    // Note: We only mark tasks as blocked on non-zero exit.
    // Exit code 0 doesn't mean the task is done - the user may have told
    // Claude to exit before completing. Tasks should be explicitly closed.
    for exited in newly_exited {
        if let Some(task_id) = exited.task_id {
            if exited.exit_code != 0 {
                println!("Agent {} exited with code {}, marking task {} as blocked",
                         exited.agent_id, exited.exit_code, task_id);

                if let Err(e) = beads::update_task_status(&task_id, "blocked") {
                    println!("Warning: Failed to update task {} status: {}", task_id, e);
                }
            } else {
                println!("Agent {} exited cleanly (code 0), task {} remains in_progress",
                         exited.agent_id, task_id);
            }
        }
    }

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

// ============== Beads Integration ==============

/// Check if Beads CLI is available
#[tauri::command]
fn beads_available() -> bool {
    beads::is_available()
}

/// Get ready tasks from Beads
#[tauri::command]
fn get_ready_tasks() -> Result<Vec<BeadsTask>, String> {
    beads::get_ready_tasks()
}

/// Get a specific task by ID
#[tauri::command]
fn get_task(task_id: String) -> Result<Option<BeadsTask>, String> {
    beads::get_task(&task_id)
}

/// Update task status
#[tauri::command]
fn update_task_status(task_id: String, status: String) -> Result<(), String> {
    beads::update_task_status(&task_id, &status)
}

/// Complete a task (close it)
#[tauri::command]
fn complete_task(task_id: String, agent_id: String) -> Result<(), String> {
    beads::complete_task(&task_id, &agent_id)
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
            // Beads integration
            beads_available,
            get_ready_tasks,
            get_task,
            update_task_status,
            complete_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
