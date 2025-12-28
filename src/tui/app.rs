//! Main TUI application state and rendering

use super::ViewMode;
use crate::agent::{AgentRegistry, AgentSession};

/// Main application state
pub struct App {
    /// Current view mode
    pub view_mode: ViewMode,
    /// Agent registry
    pub registry: AgentRegistry,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Currently selected agent index (in symphony view)
    pub selected_agent: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            view_mode: ViewMode::Symphony,
            registry: AgentRegistry::new(),
            should_quit: false,
            selected_agent: 0,
        }
    }

    /// Toggle between symphony and focus view
    pub fn toggle_view(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Symphony => ViewMode::Focus(self.selected_agent),
            ViewMode::Focus(_) => ViewMode::Symphony,
        };
    }

    /// Zoom into a specific agent
    pub fn zoom_in(&mut self, agent_index: usize) {
        self.view_mode = ViewMode::Focus(agent_index);
    }

    /// Zoom out to symphony view
    pub fn zoom_out(&mut self) {
        self.view_mode = ViewMode::Symphony;
    }

    /// Select next agent
    pub fn next_agent(&mut self) {
        let count = self.registry.active_sessions().len();
        if count > 0 {
            self.selected_agent = (self.selected_agent + 1) % count;
        }
    }

    /// Select previous agent
    pub fn prev_agent(&mut self) {
        let count = self.registry.active_sessions().len();
        if count > 0 {
            self.selected_agent = self.selected_agent.checked_sub(1).unwrap_or(count - 1);
        }
    }

    /// Get the currently selected agent session
    pub fn selected_session(&self) -> Option<&AgentSession> {
        self.registry
            .active_sessions()
            .get(self.selected_agent)
            .copied()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
