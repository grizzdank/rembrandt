//! Beads-rust integration - task tracking via `br` CLI

use super::Integration;
use crate::Result;
use std::process::Command;

/// Integration with Beads issue tracker
pub struct BeadsIntegration {
    available: bool,
}

impl BeadsIntegration {
    pub fn new() -> Self {
        let available = Command::new("br")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self { available }
    }

    /// Get ready tasks (no blockers)
    pub fn ready_tasks(&self) -> Result<Vec<BeadsTask>> {
        if !self.available {
            return Ok(vec![]);
        }

        let output = Command::new("br")
            .args(["ready", "--json"])
            .output()?;

        if output.status.success() {
            let tasks: Vec<BeadsTask> = serde_json::from_slice(&output.stdout)
                .unwrap_or_default();
            Ok(tasks)
        } else {
            Ok(vec![])
        }
    }

    /// Update task status
    pub fn update_status(&self, task_id: &str, status: &str) -> Result<()> {
        if !self.available {
            return Ok(());
        }

        Command::new("br")
            .args(["update", task_id, "--status", status])
            .output()?;

        Ok(())
    }

    /// Sync with remote
    pub fn sync(&self) -> Result<()> {
        if !self.available {
            return Ok(());
        }

        Command::new("br")
            .arg("sync")
            .output()?;

        Ok(())
    }
}

impl Default for BeadsIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl Integration for BeadsIntegration {
    fn is_available(&self) -> bool {
        self.available
    }

    fn name(&self) -> &'static str {
        "beads"
    }
}

/// A Beads task
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BeadsTask {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: Option<i32>,
}
