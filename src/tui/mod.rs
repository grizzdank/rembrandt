//! Terminal UI components
//!
//! Provides the zoom in/out interface for agent orchestration.

mod app;

pub use app::*;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// View mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Symphony view - see all agents at once (zoom out)
    Symphony,
    /// Focus view - interact with a single agent (zoom in)
    Focus(usize),
}

/// Split the terminal into the main layout areas
pub fn main_layout(area: Rect) -> Vec<Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Status bar
        ])
        .split(area)
        .to_vec()
}

/// Split the main content area for symphony view
pub fn symphony_layout(area: Rect, agent_count: usize) -> Vec<Rect> {
    let constraints: Vec<Constraint> = (0..agent_count)
        .map(|_| Constraint::Ratio(1, agent_count as u32))
        .collect();

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
        .to_vec()
}
