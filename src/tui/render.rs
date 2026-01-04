//! TUI rendering with ratatui

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::app::AGENT_TYPES;
use super::{App, ViewMode};
use crate::daemon::SessionStatus;

/// Render the entire application
pub fn render(frame: &mut Frame, app: &App) {
    // Render base view
    match app.view_mode {
        ViewMode::Symphony => render_symphony(frame, app),
        ViewMode::Solo(idx) => render_solo(frame, app, idx),
    }

    // Render overlays on top
    if app.spawn_picker.is_some() {
        render_spawn_picker(frame, app);
    }

    if app.show_help {
        render_help_overlay(frame, app);
    }
}

/// Render symphony view (overview of all agents)
fn render_symphony(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Session list
            Constraint::Length(3),  // Status bar
        ])
        .split(frame.area());

    // Header
    let attention = app.attention_count();
    let header_text = if attention > 0 {
        format!(" Rembrandt  {} agents  {} need attention ",
            app.sessions.total_count(), attention)
    } else {
        format!(" Rembrandt  {} agents ", app.sessions.total_count())
    };

    let header = Paragraph::new(header_text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(header, chunks[0]);

    // Session list
    let sessions = app.session_list();
    let total = sessions.len();

    if sessions.is_empty() {
        let empty = Paragraph::new("No agents running. Press 's' to spawn one.")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default()
                .title(" Sessions ")
                .borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
    } else {
        let now = chrono::Utc::now();
        let items: Vec<ListItem> = sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let (icon, status_text) = App::status_display(&session.status);

                let style = match &session.status {
                    SessionStatus::Running => Style::default().fg(Color::Green),
                    SessionStatus::Exited(0) => Style::default().fg(Color::Gray),
                    SessionStatus::Exited(_) => Style::default().fg(Color::Red),
                    SessionStatus::Failed(_) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                };

                let selected = if i == app.selected_index { "▶ " } else { "  " };

                // Calculate age
                let age = now.signed_duration_since(session.created_at);
                let age_str = App::format_duration(age);

                let line = Line::from(vec![
                    Span::raw(selected),
                    Span::styled(icon, style),
                    Span::raw(" "),
                    Span::styled(&session.agent_id, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(status_text, style),
                    Span::raw("  "),
                    Span::styled(&session.command, Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(age_str, Style::default().fg(Color::Cyan)),
                ]);

                ListItem::new(line)
            })
            .collect();

        // Build title with scroll indicator
        let scroll_indicator = format!(" Sessions  ↕ {}/{} ", app.selected_index + 1, total);

        let list = List::new(items)
            .block(Block::default()
                .title(scroll_indicator)
                .borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::DarkGray));

        let mut state = ListState::default();
        state.select(Some(app.selected_index));
        frame.render_stateful_widget(list, chunks[1], &mut state);
    }

    // Status bar
    let status_text = app.status_message.as_deref().unwrap_or("Press '?' for help");
    let status = Paragraph::new(format!(" {} ", status_text))
        .style(Style::default().fg(Color::White).bg(Color::Blue));
    frame.render_widget(status, chunks[2]);
}

/// Render solo view (single agent, full screen)
fn render_solo(frame: &mut Frame, app: &App, session_idx: usize) {
    let sessions = app.session_list();
    let session = match sessions.get(session_idx) {
        Some(s) => s,
        None => {
            // Session no longer exists, show message
            let msg = Paragraph::new("Session no longer exists. Press Esc to return.")
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(msg, frame.area());
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // PTY output area
            Constraint::Length(1),  // Status
        ])
        .split(frame.area());

    // Header with session info
    let (icon, status_text) = App::status_display(&session.status);
    let header = Paragraph::new(format!(" {} {} - {} ({}) ",
        icon, session.agent_id, session.command, status_text))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray))
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(header, chunks[0]);

    // PTY output area - display buffered output
    let output_block = Block::default()
        .title(format!(" {} ", session.workdir))
        .borders(Borders::ALL);

    // Get the PTY output
    let output_text = app.session_output(&session.id);
    let output_text = if output_text.is_empty() {
        format!(
            "Agent: {}\nCommand: {}\nStatus: {:?}\n\n[Waiting for output...]",
            session.agent_id, session.command, session.status
        )
    } else {
        // Show the last N lines that fit in the view
        // Keep last ~100 lines max for display
        let lines: Vec<&str> = output_text.lines().collect();
        let visible_lines = chunks[1].height.saturating_sub(2) as usize; // Account for borders
        let start = lines.len().saturating_sub(visible_lines.max(100));
        lines[start..].join("\n")
    };

    let output = Paragraph::new(output_text)
        .block(output_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(output, chunks[1]);

    // Status bar - show help, plus message if any
    let help = "Esc: back │ n: nudge │ k: kill │ ?: help";
    let status_text = match &app.status_message {
        Some(msg) => format!(" {} │ {} ", help, msg),
        None => format!(" {} ", help),
    };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Black).bg(Color::Gray));
    frame.render_widget(status, chunks[2]);
}

/// Render centered popup area
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Render help overlay
fn render_help_overlay(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 70, frame.area());

    // Clear the area first
    frame.render_widget(Clear, area);

    let help_text = match app.view_mode {
        ViewMode::Symphony => vec![
            Line::from(vec![
                Span::styled("Symphony View", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Navigation", Style::default().fg(Color::Yellow)),
            ]),
            Line::from("  j/↓     Next session"),
            Line::from("  k/↑     Previous session"),
            Line::from("  Enter   Zoom into session (Solo view)"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions", Style::default().fg(Color::Yellow)),
            ]),
            Line::from("  s       Spawn new agent"),
            Line::from("  n       Nudge selected agent"),
            Line::from("  K/Del   Kill selected agent"),
            Line::from("  c       Cleanup completed sessions"),
            Line::from(""),
            Line::from(vec![
                Span::styled("General", Style::default().fg(Color::Yellow)),
            ]),
            Line::from("  ?       Toggle this help"),
            Line::from("  q       Quit"),
            Line::from("  Ctrl+C  Quit"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press any key to close", Style::default().fg(Color::DarkGray)),
            ]),
        ],
        ViewMode::Solo(_) => vec![
            Line::from(vec![
                Span::styled("Solo View", Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions", Style::default().fg(Color::Yellow)),
            ]),
            Line::from("  Esc     Return to Symphony view"),
            Line::from("  n       Nudge agent"),
            Line::from("  k/K     Kill agent"),
            Line::from(""),
            Line::from(vec![
                Span::styled("General", Style::default().fg(Color::Yellow)),
            ]),
            Line::from("  ?       Toggle this help"),
            Line::from("  Ctrl+Q  Quit"),
            Line::from(""),
            Line::from(vec![
                Span::styled("Press any key to close", Style::default().fg(Color::DarkGray)),
            ]),
        ],
    };

    let help = Paragraph::new(help_text)
        .block(Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black)))
        .style(Style::default().fg(Color::White).bg(Color::Black))
        .wrap(Wrap { trim: false });

    frame.render_widget(help, area);
}

/// Render spawn picker dialog
fn render_spawn_picker(frame: &mut Frame, app: &App) {
    let picker = match &app.spawn_picker {
        Some(p) => p,
        None => return,
    };

    let area = centered_rect(50, 50, frame.area());

    // Clear the area first
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = AGENT_TYPES
        .iter()
        .enumerate()
        .map(|(i, (short, name))| {
            let selected = if i == picker.selected { "▶ " } else { "  " };
            let style = if i == picker.selected {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::raw(selected),
                Span::styled(*name, style),
                Span::styled(format!(" ({})", short), Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .title(" Spawn Agent (Enter to confirm, Esc to cancel) ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black)))
        .style(Style::default().fg(Color::White).bg(Color::Black));

    frame.render_widget(list, area);
}
