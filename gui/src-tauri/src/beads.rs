//! Beads integration - task tracking via `bd` CLI
//!
//! Provides access to the Beads issue tracker for task assignment to agents.

use serde::{Deserialize, Serialize};
use std::process::Command;

/// A Beads task from the issue tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsTask {
    pub id: String,
    pub title: String,
    pub status: String,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub issue_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Check if the beads CLI is available
pub fn is_available() -> bool {
    Command::new("bd")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get ready tasks (no blockers) from Beads
pub fn get_ready_tasks() -> Result<Vec<BeadsTask>, String> {
    let output = Command::new("bd")
        .args(["ready", "--json"])
        .output()
        .map_err(|e| format!("Failed to run bd: {}", e))?;

    if output.status.success() {
        let tasks: Vec<BeadsTask> = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse bd output: {}", e))?;
        Ok(tasks)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("bd ready failed: {}", stderr))
    }
}

/// Get a specific task by ID
pub fn get_task(task_id: &str) -> Result<Option<BeadsTask>, String> {
    let output = Command::new("bd")
        .args(["show", task_id, "--json"])
        .output()
        .map_err(|e| format!("Failed to run bd: {}", e))?;

    if output.status.success() {
        let task: BeadsTask = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse bd output: {}", e))?;
        Ok(Some(task))
    } else {
        Ok(None)
    }
}

/// Update task status
pub fn update_task_status(task_id: &str, status: &str) -> Result<(), String> {
    let output = Command::new("bd")
        .args(["update", task_id, "--status", status])
        .output()
        .map_err(|e| format!("Failed to run bd: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("bd update failed: {}", stderr))
    }
}

/// Assign task to an agent (sets status to in_progress)
pub fn claim_task(task_id: &str, agent_id: &str) -> Result<(), String> {
    // First update status to in_progress
    update_task_status(task_id, "in_progress")?;

    // Add a comment noting which agent claimed it
    let comment = format!("Claimed by agent: {}", agent_id);
    let _ = Command::new("bd")
        .args(["comments", task_id, "--add", &comment])
        .output();

    Ok(())
}

/// Mark task as completed
pub fn complete_task(task_id: &str, agent_id: &str) -> Result<(), String> {
    // Add completion comment
    let comment = format!("Completed by agent: {}", agent_id);
    let _ = Command::new("bd")
        .args(["comments", task_id, "--add", &comment])
        .output();

    // Close the task
    let output = Command::new("bd")
        .args(["close", task_id])
        .output()
        .map_err(|e| format!("Failed to run bd: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("bd close failed: {}", stderr))
    }
}
