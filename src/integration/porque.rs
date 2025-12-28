//! Porque integration - architectural decision context via `pq` CLI

use super::Integration;
use crate::Result;
use std::path::Path;
use std::process::Command;

/// Integration with Porque ADR system
pub struct PorqueIntegration {
    available: bool,
}

impl PorqueIntegration {
    pub fn new() -> Self {
        let available = Command::new("pq")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self { available }
    }

    /// Get decisions relevant to a path
    pub fn context(&self, path: &Path) -> Result<Vec<Decision>> {
        if !self.available {
            return Ok(vec![]);
        }

        let output = Command::new("pq")
            .args(["context", "--json"])
            .arg(path)
            .output()?;

        if output.status.success() {
            let decisions: Vec<Decision> = serde_json::from_slice(&output.stdout)
                .unwrap_or_default();
            Ok(decisions)
        } else {
            Ok(vec![])
        }
    }

    /// Check if changes violate any decisions
    pub fn check(&self, files: &[&Path]) -> Result<Vec<Violation>> {
        if !self.available {
            return Ok(vec![]);
        }

        let mut cmd = Command::new("pq");
        cmd.args(["check", "--json"]);
        for file in files {
            cmd.arg(file);
        }

        let output = cmd.output()?;

        if output.status.success() {
            let violations: Vec<Violation> = serde_json::from_slice(&output.stdout)
                .unwrap_or_default();
            Ok(violations)
        } else {
            Ok(vec![])
        }
    }
}

impl Default for PorqueIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl Integration for PorqueIntegration {
    fn is_available(&self) -> bool {
        self.available
    }

    fn name(&self) -> &'static str {
        "porque"
    }
}

/// An architectural decision
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Decision {
    pub id: String,
    pub title: String,
    pub status: String,
    pub context: Option<String>,
}

/// A decision violation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Violation {
    pub decision_id: String,
    pub file: String,
    pub reason: String,
}
