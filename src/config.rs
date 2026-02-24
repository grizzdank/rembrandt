//! Rembrandt configuration for v2 orchestration paths.

/// Workspace isolation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefaultIsolationMode {
    Branch,
    Worktree,
}

/// Preferred terminal backend for attach/observe flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalBackendKind {
    None,
    Tmux,
    Cmux,
}

/// Runtime config for v2 services.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub default_spawn_isolation: DefaultIsolationMode,
    pub default_compete_isolation: DefaultIsolationMode,
    pub csi_poll_interval_secs: u64,
    pub terminal_backend: TerminalBackendKind,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_spawn_isolation: DefaultIsolationMode::Branch,
            default_compete_isolation: DefaultIsolationMode::Worktree,
            csi_poll_interval_secs: 15,
            terminal_backend: TerminalBackendKind::None,
        }
    }
}
