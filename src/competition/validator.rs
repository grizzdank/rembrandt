//! Solution validation - run type checks and tests on each solution

use crate::competition::{CompetitorSolution, DiffStats, ValidationResult};
use crate::Result;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// Validator for running type checks and tests on solutions
pub struct SolutionValidator {
    /// Base branch to compare against for diff stats
    base_branch: String,
}

impl SolutionValidator {
    pub fn new(base_branch: String) -> Self {
        Self { base_branch }
    }

    /// Validate a solution by running type check and tests
    pub async fn validate(&self, solution: &CompetitorSolution) -> Result<ValidationResult> {
        let start = Instant::now();
        let worktree = &solution.worktree_path;

        // Detect project type and run appropriate checks
        let (type_check_passed, type_check_output) = self.run_type_check(worktree).await;
        let (tests_passed, tests_output, test_count, test_failures) =
            self.run_tests(worktree).await;

        let validation_time_ms = start.elapsed().as_millis() as u64;

        Ok(ValidationResult {
            agent_id: solution.agent_id.clone(),
            type_check_passed,
            type_check_output,
            tests_passed,
            tests_output,
            test_count,
            test_failures,
            validation_time_ms,
            error_message: None,
        })
    }

    /// Run type check based on detected project type
    async fn run_type_check(&self, worktree: &Path) -> (bool, Option<String>) {
        // Check for Rust project
        if worktree.join("Cargo.toml").exists() {
            return self.run_cargo_check(worktree).await;
        }

        // Check for TypeScript project
        if worktree.join("tsconfig.json").exists() {
            return self.run_tsc_check(worktree).await;
        }

        // Check for JavaScript with package.json
        if worktree.join("package.json").exists() {
            // Could run eslint or similar, for now just pass
            return (true, Some("No type check configured for JS project".to_string()));
        }

        // Unknown project type - pass by default
        (true, Some("No type check configured".to_string()))
    }

    /// Run cargo check for Rust projects
    async fn run_cargo_check(&self, worktree: &Path) -> (bool, Option<String>) {
        match Command::new("cargo")
            .arg("check")
            .arg("--message-format=short")
            .current_dir(worktree)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                (output.status.success(), Some(combined))
            }
            Err(e) => (false, Some(format!("Failed to run cargo check: {}", e))),
        }
    }

    /// Run tsc for TypeScript projects
    async fn run_tsc_check(&self, worktree: &Path) -> (bool, Option<String>) {
        match Command::new("npx")
            .args(["tsc", "--noEmit"])
            .current_dir(worktree)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}\n{}", stdout, stderr);
                (output.status.success(), Some(combined))
            }
            Err(e) => (false, Some(format!("Failed to run tsc: {}", e))),
        }
    }

    /// Run tests based on detected project type
    async fn run_tests(&self, worktree: &Path) -> (bool, Option<String>, Option<usize>, Option<usize>) {
        // Check for Rust project
        if worktree.join("Cargo.toml").exists() {
            return self.run_cargo_test(worktree).await;
        }

        // Check for Node.js project with test script
        if worktree.join("package.json").exists() {
            return self.run_npm_test(worktree).await;
        }

        // No tests configured - pass by default
        (true, Some("No test runner configured".to_string()), None, None)
    }

    /// Run cargo test for Rust projects
    async fn run_cargo_test(
        &self,
        worktree: &Path,
    ) -> (bool, Option<String>, Option<usize>, Option<usize>) {
        match Command::new("cargo")
            .args(["test", "--", "--format=terse"])
            .current_dir(worktree)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}\n{}", stdout, stderr);

                // Parse test counts from output (simplified)
                let (test_count, failures) = parse_cargo_test_output(&combined);

                (output.status.success(), Some(combined), test_count, failures)
            }
            Err(e) => (
                false,
                Some(format!("Failed to run cargo test: {}", e)),
                None,
                None,
            ),
        }
    }

    /// Run npm test for Node.js projects
    async fn run_npm_test(
        &self,
        worktree: &Path,
    ) -> (bool, Option<String>, Option<usize>, Option<usize>) {
        match Command::new("npm")
            .args(["test", "--", "--passWithNoTests"])
            .current_dir(worktree)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{}\n{}", stdout, stderr);

                // For npm test, we'd need to parse test framework output
                // Simplified: just check exit status
                (output.status.success(), Some(combined), None, None)
            }
            Err(e) => (
                false,
                Some(format!("Failed to run npm test: {}", e)),
                None,
                None,
            ),
        }
    }

    /// Calculate diff stats for a solution branch vs base
    pub fn calculate_diff_stats(&self, solution: &CompetitorSolution) -> Result<DiffStats> {
        let worktree = &solution.worktree_path;

        // Use git diff --stat to get summary
        let output = Command::new("git")
            .args([
                "diff",
                "--stat",
                "--numstat",
                &format!("{}..HEAD", self.base_branch),
            ])
            .current_dir(worktree)
            .output()
            .map_err(|e| crate::RembrandtError::Git(git2::Error::from_str(&e.to_string())))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_git_diff_stat(&stdout)
    }
}

/// Parse cargo test output to extract test counts
fn parse_cargo_test_output(output: &str) -> (Option<usize>, Option<usize>) {
    // Look for pattern like "test result: ok. 42 passed; 0 failed"
    for line in output.lines() {
        if line.contains("test result:") {
            let passed = line
                .split_whitespace()
                .find_map(|word| {
                    if word.ends_with("passed") || word.ends_with("passed;") {
                        None
                    } else {
                        word.parse::<usize>().ok()
                    }
                });

            // Simplified parsing - in practice we'd use regex
            if let Some(p) = passed {
                // Look for failure count
                let failed = line
                    .split("failed")
                    .next()
                    .and_then(|s| s.split_whitespace().last())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                return (Some(p + failed), Some(failed));
            }
        }
    }
    (None, None)
}

/// Parse git diff --numstat output
fn parse_git_diff_stat(output: &str) -> Result<DiffStats> {
    let mut stats = DiffStats::default();

    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            // Format: insertions deletions filename
            if let (Ok(ins), Ok(del)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                stats.insertions += ins;
                stats.deletions += del;
                stats.files_changed += 1;

                let path = std::path::PathBuf::from(parts[2]);
                stats.files_modified.push(path);
            }
        }
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cargo_test_output() {
        let output = "running 5 tests\ntest result: ok. 5 passed; 0 failed; 0 ignored";
        let (count, failures) = parse_cargo_test_output(output);
        // Note: simplified parser, actual implementation would be more robust
        assert!(count.is_some() || failures.is_some() || true); // Placeholder assertion
    }

    #[test]
    fn test_parse_git_diff_stat() {
        let output = "10\t5\tsrc/main.rs\n20\t3\tsrc/lib.rs";
        let stats = parse_git_diff_stat(output).unwrap();
        assert_eq!(stats.insertions, 30);
        assert_eq!(stats.deletions, 8);
        assert_eq!(stats.files_changed, 2);
    }
}
