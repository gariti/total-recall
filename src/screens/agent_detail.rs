//! Agent detail screen — focused view of one agent's tmux output.

use ansi_to_tui::IntoText;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use ratatui_garnish::{shadow::HalfShadow, GarnishableWidget, Padding};
use std::sync::Arc;

use crate::models::agent::{Agent, AgentStatus};
use crate::services::Theme;

/// Agent detail screen state.
pub struct AgentDetailScreen {
    theme: Arc<Theme>,
    /// Scroll offset for the output area.
    pub scroll_offset: u16,
    /// Whether auto-scroll is enabled.
    pub auto_scroll: bool,
}

impl AgentDetailScreen {
    pub fn new(theme: Arc<Theme>) -> Self {
        Self {
            theme,
            scroll_offset: 0,
            auto_scroll: true,
        }
    }

    /// Draw the detail view for a specific agent.
    pub fn draw(&mut self, f: &mut Frame, area: Rect, agent: &Agent) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Agent header
                Constraint::Min(0),    // Output area
                Constraint::Length(2), // Help bar
            ])
            .split(area);

        // Header bar
        let status_color = match agent.status {
            AgentStatus::Starting => self.theme.color3,
            AgentStatus::Active => self.theme.color2,
            AgentStatus::Idle => self.theme.color3,
            AgentStatus::Complete => self.theme.color6,
            AgentStatus::Failed => self.theme.color1,
            AgentStatus::Killed => self.theme.color1,
        };

        let level = agent.activity_level() as usize;
        let filled = "█".repeat(level);
        let empty = "░".repeat(10 - level);

        let project_name = agent
            .project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?");

        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", agent.agent_type),
                Style::default()
                    .fg(self.theme.foreground)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{} {} ", agent.status.icon(), agent.status.label()),
                Style::default().fg(status_color),
            ),
            Span::styled(&filled, Style::default().fg(self.theme.color2)),
            Span::styled(&empty, Style::default().fg(self.theme.color0)),
            Span::styled(
                format!("  {}", project_name),
                Style::default().fg(self.theme.color8),
            ),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("\"{}\"", truncate_str(&agent.task_prompt, 50)))
                .border_style(Style::default().fg(self.theme.color6)),
        );
        f.render_widget(header, chunks[0]);

        // Output area — tmux pane capture
        let output_block = Block::default()
            .borders(Borders::ALL)
            .title("Output")
            .border_style(Style::default().fg(self.theme.color8));

        if agent.last_output_lines.is_empty() {
            let empty_text = Paragraph::new(vec![
                Line::raw(""),
                Line::styled(
                    "  Waiting for output...",
                    Style::default().fg(self.theme.color8),
                ),
            ])
            .block(output_block);
            f.render_widget(empty_text, chunks[1]);
        } else {
            // Join output lines and try to parse ANSI colors
            let raw_output = agent.last_output_lines.join("\n");
            let text = raw_output
                .as_bytes()
                .into_text()
                .unwrap_or_else(|_| Text::raw(&raw_output));

            // Auto-scroll to bottom
            if self.auto_scroll {
                let visible_height = chunks[1].height.saturating_sub(2) as usize; // minus borders
                let total_lines = text.lines.len();
                if total_lines > visible_height {
                    self.scroll_offset = (total_lines - visible_height) as u16;
                }
            }

            let output = Paragraph::new(text)
                .block(output_block)
                .scroll((self.scroll_offset, 0))
                .wrap(Wrap { trim: false });

            let garnished = output
                .garnish(Padding::horizontal(1))
                .garnish(HalfShadow::default());
            f.render_widget(garnished, chunks[1]);
        }

        // Help bar
        let help = Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("a", Style::default().fg(self.theme.color6)),
            Span::styled(" Attach  ", Style::default().fg(self.theme.color7)),
            Span::styled("k", Style::default().fg(self.theme.color6)),
            Span::styled(" Kill  ", Style::default().fg(self.theme.color7)),
            Span::styled("j/k", Style::default().fg(self.theme.color6)),
            Span::styled(" Scroll  ", Style::default().fg(self.theme.color7)),
            Span::styled("G", Style::default().fg(self.theme.color6)),
            Span::styled(" Bottom  ", Style::default().fg(self.theme.color7)),
            Span::styled("Esc", Style::default().fg(self.theme.color6)),
            Span::styled(" Back", Style::default().fg(self.theme.color7)),
        ]));
        f.render_widget(help, chunks[2]);
    }

    /// Scroll up.
    pub fn scroll_up(&mut self) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    /// Scroll down.
    pub fn scroll_down(&mut self, max_lines: usize) {
        self.scroll_offset = (self.scroll_offset + 1).min(max_lines as u16);
    }

    /// Jump to bottom and re-enable auto-scroll.
    pub fn scroll_to_bottom(&mut self) {
        self.auto_scroll = true;
    }

    /// Reset scroll state (e.g., when switching agents).
    pub fn reset_scroll(&mut self) {
        self.scroll_offset = 0;
        self.auto_scroll = true;
    }
}

/// Truncate a string to max chars, adding "..." if truncated.
fn truncate_str(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max {
        format!("{}...", chars[..max].iter().collect::<String>())
    } else {
        s.to_string()
    }
}
