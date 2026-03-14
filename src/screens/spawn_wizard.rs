//! Spawn wizard — multi-step popup for creating new agents.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::sync::Arc;

use crate::models::agent_registry::AgentRegistry;
use crate::models::Project;
use crate::services::Theme;

/// Which step of the wizard we're on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    PickProject,
    PickAgentType,
    EnterPrompt,
    ToggleWorktree,
    Confirm,
}

/// Result of the wizard when the user confirms.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub project_path: String,
    pub agent_type: String,
    pub task_prompt: String,
    pub use_worktree: bool,
}

/// Spawn wizard state.
pub struct SpawnWizard {
    theme: Arc<Theme>,
    pub step: WizardStep,
    pub active: bool,

    // Step 1: Project selection
    pub projects: Vec<Project>,
    pub project_state: ListState,

    // Step 2: Agent type selection
    pub agent_types: Vec<(String, String)>, // (key, display_name)
    pub type_state: ListState,

    // Step 3: Prompt input
    pub prompt_text: String,

    // Step 4: Worktree toggle
    pub use_worktree: bool,
}

impl SpawnWizard {
    pub fn new(theme: Arc<Theme>) -> Self {
        Self {
            theme,
            step: WizardStep::PickProject,
            active: false,
            projects: Vec::new(),
            project_state: ListState::default(),
            agent_types: Vec::new(),
            type_state: ListState::default(),
            prompt_text: String::new(),
            use_worktree: true,
        }
    }

    /// Open the wizard with available projects and agent types.
    pub fn open(&mut self, projects: Vec<Project>, registry: &AgentRegistry) {
        self.active = true;
        self.step = WizardStep::PickProject;
        self.projects = projects;
        self.project_state.select(Some(0));

        // Build agent type list
        self.agent_types = vec![("general-purpose".to_string(), "General Purpose".to_string())];
        for key in registry.type_keys() {
            if key != "general-purpose" {
                if let Some(entry) = registry.get(&key) {
                    self.agent_types.push((key, entry.name.clone()));
                }
            }
        }
        self.type_state.select(Some(0));
        self.prompt_text.clear();
        self.use_worktree = true;
    }

    /// Close the wizard without spawning.
    pub fn close(&mut self) {
        self.active = false;
    }

    /// Handle a key event. Returns Some(SpawnRequest) when user confirms.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SpawnRequest> {
        match key.code {
            KeyCode::Esc => {
                if self.step == WizardStep::PickProject {
                    self.close();
                } else {
                    // Go back one step
                    self.step = match self.step {
                        WizardStep::PickAgentType => WizardStep::PickProject,
                        WizardStep::EnterPrompt => WizardStep::PickAgentType,
                        WizardStep::ToggleWorktree => WizardStep::EnterPrompt,
                        WizardStep::Confirm => WizardStep::ToggleWorktree,
                        WizardStep::PickProject => WizardStep::PickProject,
                    };
                }
                None
            }
            KeyCode::Enter => {
                match self.step {
                    WizardStep::PickProject => {
                        self.step = WizardStep::PickAgentType;
                        None
                    }
                    WizardStep::PickAgentType => {
                        self.step = WizardStep::EnterPrompt;
                        None
                    }
                    WizardStep::EnterPrompt => {
                        if !self.prompt_text.trim().is_empty() {
                            self.step = WizardStep::ToggleWorktree;
                        }
                        None
                    }
                    WizardStep::ToggleWorktree => {
                        self.step = WizardStep::Confirm;
                        None
                    }
                    WizardStep::Confirm => {
                        // Build spawn request
                        let project_idx = self.project_state.selected().unwrap_or(0);
                        let type_idx = self.type_state.selected().unwrap_or(0);

                        let project_path = self.projects.get(project_idx)
                            .map(|p| p.decoded_path.clone())
                            .unwrap_or_default();
                        let agent_type = self.agent_types.get(type_idx)
                            .map(|(k, _)| k.clone())
                            .unwrap_or_else(|| "general-purpose".to_string());

                        self.close();

                        Some(SpawnRequest {
                            project_path,
                            agent_type,
                            task_prompt: self.prompt_text.clone(),
                            use_worktree: self.use_worktree,
                        })
                    }
                }
            }
            // Navigation for list steps
            KeyCode::Up | KeyCode::Char('k') => {
                match self.step {
                    WizardStep::PickProject => {
                        if let Some(sel) = self.project_state.selected() {
                            let new = if sel == 0 { self.projects.len().saturating_sub(1) } else { sel - 1 };
                            self.project_state.select(Some(new));
                        }
                    }
                    WizardStep::PickAgentType => {
                        if let Some(sel) = self.type_state.selected() {
                            let new = if sel == 0 { self.agent_types.len().saturating_sub(1) } else { sel - 1 };
                            self.type_state.select(Some(new));
                        }
                    }
                    _ => {}
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match self.step {
                    WizardStep::PickProject => {
                        if let Some(sel) = self.project_state.selected() {
                            let new = if sel >= self.projects.len().saturating_sub(1) { 0 } else { sel + 1 };
                            self.project_state.select(Some(new));
                        }
                    }
                    WizardStep::PickAgentType => {
                        if let Some(sel) = self.type_state.selected() {
                            let new = if sel >= self.agent_types.len().saturating_sub(1) { 0 } else { sel + 1 };
                            self.type_state.select(Some(new));
                        }
                    }
                    _ => {}
                }
                None
            }
            // Text input for prompt step
            KeyCode::Char(c) => {
                if self.step == WizardStep::EnterPrompt {
                    self.prompt_text.push(c);
                } else if self.step == WizardStep::ToggleWorktree && (c == 'w' || c == ' ') {
                    self.use_worktree = !self.use_worktree;
                }
                None
            }
            KeyCode::Backspace => {
                if self.step == WizardStep::EnterPrompt {
                    self.prompt_text.pop();
                }
                None
            }
            _ => None,
        }
    }

    /// Draw the wizard as a centered popup overlay.
    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        if !self.active {
            return;
        }

        // Center the popup (60% width, 70% height)
        let popup_area = centered_rect(60, 70, area);

        // Clear the area behind the popup
        f.render_widget(Clear, popup_area);

        let popup_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Spawn Agent — Step {} of 5 ",
                match self.step {
                    WizardStep::PickProject => 1,
                    WizardStep::PickAgentType => 2,
                    WizardStep::EnterPrompt => 3,
                    WizardStep::ToggleWorktree => 4,
                    WizardStep::Confirm => 5,
                }
            ))
            .border_style(Style::default().fg(self.theme.color6));

        let inner = popup_block.inner(popup_area);
        f.render_widget(popup_block, popup_area);

        match self.step {
            WizardStep::PickProject => self.draw_project_step(f, inner),
            WizardStep::PickAgentType => self.draw_type_step(f, inner),
            WizardStep::EnterPrompt => self.draw_prompt_step(f, inner),
            WizardStep::ToggleWorktree => self.draw_worktree_step(f, inner),
            WizardStep::Confirm => self.draw_confirm_step(f, inner),
        }
    }

    fn draw_project_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area);

        let hint = Paragraph::new(Line::from(vec![
            Span::styled(" Select a project:", Style::default().fg(self.theme.color7)),
        ]));
        f.render_widget(hint, chunks[0]);

        let items: Vec<ListItem> = self.projects
            .iter()
            .map(|p| {
                ListItem::new(Line::from(vec![
                    Span::styled(&p.display_name, Style::default().fg(self.theme.foreground)),
                    Span::styled(
                        format!("  {}", p.decoded_path),
                        Style::default().fg(self.theme.color8),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(self.theme.color8).add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
        f.render_stateful_widget(list, chunks[1], &mut self.project_state);
    }

    fn draw_type_step(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area);

        let hint = Paragraph::new(Line::from(vec![
            Span::styled(" Select agent type:", Style::default().fg(self.theme.color7)),
        ]));
        f.render_widget(hint, chunks[0]);

        let items: Vec<ListItem> = self.agent_types
            .iter()
            .map(|(key, name)| {
                ListItem::new(Line::from(vec![
                    Span::styled(key, Style::default().fg(self.theme.color6).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("  — {}", name), Style::default().fg(self.theme.color8)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(self.theme.color8).add_modifier(Modifier::BOLD))
            .highlight_symbol("► ");
        f.render_stateful_widget(list, chunks[1], &mut self.type_state);
    }

    fn draw_prompt_step(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .split(area);

        let hint = Paragraph::new(Line::from(vec![
            Span::styled(" Enter task prompt:", Style::default().fg(self.theme.color7)),
        ]));
        f.render_widget(hint, chunks[0]);

        // Render the text input area
        let input_text = if self.prompt_text.is_empty() {
            Span::styled("Type your prompt here...", Style::default().fg(self.theme.color8))
        } else {
            Span::styled(&self.prompt_text, Style::default().fg(self.theme.foreground))
        };

        let input = Paragraph::new(Line::from(vec![
            Span::raw(" "),
            input_text,
            Span::styled("█", Style::default().fg(self.theme.color6)), // cursor
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.color6)),
        );
        f.render_widget(input, chunks[1]);

        let hint2 = Paragraph::new(vec![
            Line::raw(""),
            Line::styled(
                " Press Enter when done, Esc to go back",
                Style::default().fg(self.theme.color8),
            ),
        ]);
        f.render_widget(hint2, chunks[2]);
    }

    fn draw_worktree_step(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(4),
                Constraint::Min(0),
            ])
            .split(area);

        let hint = Paragraph::new(Line::from(vec![
            Span::styled(" Isolate in jj worktree?", Style::default().fg(self.theme.color7)),
        ]));
        f.render_widget(hint, chunks[0]);

        let toggle = if self.use_worktree { "[x]" } else { "[ ]" };
        let option = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("  "),
                Span::styled(toggle, Style::default().fg(self.theme.color6).add_modifier(Modifier::BOLD)),
                Span::styled(" Create isolated jj workspace", Style::default().fg(self.theme.foreground)),
            ]),
            Line::from(vec![
                Span::raw("      "),
                Span::styled(
                    "Agent works in /tmp/tr-<project>-<name>/ without touching main worktree",
                    Style::default().fg(self.theme.color8),
                ),
            ]),
        ]);
        f.render_widget(option, chunks[1]);

        let hint2 = Paragraph::new(vec![
            Line::raw(""),
            Line::styled(
                " Press Space/w to toggle, Enter to continue",
                Style::default().fg(self.theme.color8),
            ),
        ]);
        f.render_widget(hint2, chunks[2]);
    }

    fn draw_confirm_step(&self, f: &mut Frame, area: Rect) {
        let project_idx = self.project_state.selected().unwrap_or(0);
        let type_idx = self.type_state.selected().unwrap_or(0);

        let project_name = self.projects.get(project_idx)
            .map(|p| p.display_name.as_str())
            .unwrap_or("?");
        let agent_type = self.agent_types.get(type_idx)
            .map(|(k, _)| k.as_str())
            .unwrap_or("?");

        let summary = Paragraph::new(vec![
            Line::raw(""),
            Line::styled(
                " Ready to deploy:",
                Style::default().fg(self.theme.color7).add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::from(vec![
                Span::styled("  Project:   ", Style::default().fg(self.theme.color8)),
                Span::styled(project_name, Style::default().fg(self.theme.foreground)),
            ]),
            Line::from(vec![
                Span::styled("  Type:      ", Style::default().fg(self.theme.color8)),
                Span::styled(agent_type, Style::default().fg(self.theme.color6)),
            ]),
            Line::from(vec![
                Span::styled("  Prompt:    ", Style::default().fg(self.theme.color8)),
                Span::styled(&self.prompt_text, Style::default().fg(self.theme.foreground)),
            ]),
            Line::from(vec![
                Span::styled("  Worktree:  ", Style::default().fg(self.theme.color8)),
                Span::styled(
                    if self.use_worktree { "Yes (isolated)" } else { "No (direct)" },
                    Style::default().fg(self.theme.foreground),
                ),
            ]),
            Line::raw(""),
            Line::styled(
                " Press Enter to spawn, Esc to go back",
                Style::default().fg(self.theme.color2).add_modifier(Modifier::BOLD),
            ),
        ])
        .wrap(Wrap { trim: false });
        f.render_widget(summary, area);
    }
}

/// Create a centered rectangle within the given area.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
