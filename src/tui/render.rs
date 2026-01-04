//! TUI rendering with ratatui

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::{App, ViewMode};
use crate::daemon::SessionStatus;

/// Render the entire application
pub fn render(frame: &mut Frame, app: &App) {
    match app.view_mode {
        ViewMode::Symphony => render_symphony(frame, app),
        ViewMode::Solo(idx) => render_solo(frame, app, idx),
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

    if sessions.is_empty() {
        let empty = Paragraph::new("No agents running. Press 's' to spawn one.")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default()
                .title(" Sessions ")
                .borders(Borders::ALL));
        frame.render_widget(empty, chunks[1]);
    } else {
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

                let line = Line::from(vec![
                    Span::raw(selected),
                    Span::styled(icon, style),
                    Span::raw(" "),
                    Span::styled(&session.agent_id, Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(status_text, style),
                    Span::raw("  "),
                    Span::styled(&session.command, Style::default().fg(Color::DarkGray)),
                ]);

                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .title(" Sessions ")
                .borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::DarkGray));

        let mut state = ListState::default();
        state.select(Some(app.selected_index));
        frame.render_stateful_widget(list, chunks[1], &mut state);
    }

    // Status bar
    let status_text = app.status_message.as_deref().unwrap_or("");
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

    // PTY output area (placeholder - real implementation needs PTY reading)
    // TODO: Read from session's output buffer and display
    let output_block = Block::default()
        .title(format!(" {} ", session.workdir))
        .borders(Borders::ALL);

    let output_text = format!(
        "Agent: {}\nCommand: {}\nStatus: {:?}\n\n[PTY output will appear here once attach is implemented]",
        session.agent_id, session.command, session.status
    );

    let output = Paragraph::new(output_text)
        .block(output_block);
    frame.render_widget(output, chunks[1]);

    // Status bar - show help, plus message if any
    let help = "Esc: back │ n: nudge │ k: kill";
    let status_text = match &app.status_message {
        Some(msg) => format!(" {} │ {} ", help, msg),
        None => format!(" {} ", help),
    };
    let status = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Black).bg(Color::Gray));
    frame.render_widget(status, chunks[2]);
}
