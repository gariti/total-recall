//! Browser screen - main interface for browsing projects and sessions.

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::sync::Arc;

use crate::config::Config;
use crate::models::{Project, Session};
use crate::services::{copy_to_clipboard, SessionStore};

use super::Screen;

/// Which pane has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Projects,
    Sessions,
}

/// Browser screen for navigating projects and sessions.
pub struct BrowserScreen {
    session_store: Arc<SessionStore>,
    config: Arc<Config>,

    // UI state
    focus: Focus,
    project_state: ListState,
    session_state: ListState,

    // Cached data
    projects: Vec<Project>,
    current_sessions: Vec<Session>,
}

impl BrowserScreen {
    /// Create a new browser screen.
    pub fn new(session_store: Arc<SessionStore>, config: Arc<Config>) -> Self {
        let mut project_state = ListState::default();
        project_state.select(Some(0));

        Self {
            session_store,
            config,
            focus: Focus::Projects,
            project_state,
            session_state: ListState::default(),
            projects: Vec::new(),
            current_sessions: Vec::new(),
        }
    }

    /// Load all sessions from the store.
    pub async fn load_sessions(&mut self) -> anyhow::Result<()> {
        // We need mutable access to session_store, so we'll use interior mutability pattern
        // For now, let's cast away the Arc - this is safe because we only call this once at startup
        let store = Arc::get_mut(&mut self.session_store)
            .ok_or_else(|| anyhow::anyhow!("Session store is shared"))?;

        store.scan()?;
        self.projects = store.projects().to_vec();

        // Load sessions for first project
        if let Some(project) = self.projects.first() {
            if let Some(sessions) = store.sessions_for_project(&project.encoded_path) {
                self.current_sessions = sessions.clone();
                if !self.current_sessions.is_empty() {
                    self.session_state.select(Some(0));
                }
            }
        }

        Ok(())
    }

    /// Get total session count.
    pub fn session_count(&self) -> usize {
        self.session_store.total_session_count()
    }

    /// Get currently selected project.
    fn selected_project(&self) -> Option<&Project> {
        self.project_state
            .selected()
            .and_then(|i| self.projects.get(i))
    }

    /// Get currently selected session.
    fn selected_session(&self) -> Option<&Session> {
        self.session_state
            .selected()
            .and_then(|i| self.current_sessions.get(i))
    }

    /// Update sessions list when project changes.
    fn update_sessions_for_project(&mut self) {
        if let Some(project) = self.selected_project() {
            if let Some(sessions) = self.session_store.sessions_for_project(&project.encoded_path) {
                self.current_sessions = sessions.clone();
                self.session_state.select(if self.current_sessions.is_empty() {
                    None
                } else {
                    Some(0)
                });
            } else {
                self.current_sessions.clear();
                self.session_state.select(None);
            }
        }
    }

    /// Navigate up in current list.
    fn move_up(&mut self) {
        match self.focus {
            Focus::Projects => {
                if let Some(selected) = self.project_state.selected() {
                    let new_index = if selected == 0 {
                        self.projects.len().saturating_sub(1)
                    } else {
                        selected - 1
                    };
                    self.project_state.select(Some(new_index));
                    self.update_sessions_for_project();
                }
            }
            Focus::Sessions => {
                if let Some(selected) = self.session_state.selected() {
                    let new_index = if selected == 0 {
                        self.current_sessions.len().saturating_sub(1)
                    } else {
                        selected - 1
                    };
                    self.session_state.select(Some(new_index));
                }
            }
        }
    }

    /// Navigate down in current list.
    fn move_down(&mut self) {
        match self.focus {
            Focus::Projects => {
                if let Some(selected) = self.project_state.selected() {
                    let new_index = if selected >= self.projects.len().saturating_sub(1) {
                        0
                    } else {
                        selected + 1
                    };
                    self.project_state.select(Some(new_index));
                    self.update_sessions_for_project();
                }
            }
            Focus::Sessions => {
                if let Some(selected) = self.session_state.selected() {
                    let new_index = if selected >= self.current_sessions.len().saturating_sub(1) {
                        0
                    } else {
                        selected + 1
                    };
                    self.session_state.select(Some(new_index));
                }
            }
        }
    }

    /// Switch focus between panes.
    fn switch_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Projects => Focus::Sessions,
            Focus::Sessions => Focus::Projects,
        };
    }

    /// Copy resume command for selected session.
    fn copy_resume_command(&self) -> Option<String> {
        if let Some(session) = self.selected_session() {
            let cmd = session.resume_command();
            if copy_to_clipboard(&cmd).is_ok() {
                return Some(format!("Copied: {}", cmd));
            } else {
                return Some("Failed to copy to clipboard".to_string());
            }
        }
        None
    }
}

#[async_trait]
impl Screen for BrowserScreen {
    fn draw(&mut self, f: &mut Frame, area: Rect) {
        // Split into left (projects) and right (preview + sessions)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25), // Projects column
                Constraint::Percentage(75), // Preview + Sessions column
            ])
            .split(area);

        // Split right column: Preview on top (prominent), Sessions below
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Preview (prominent)
                Constraint::Percentage(40), // Sessions list
            ])
            .split(main_chunks[1]);

        // Projects pane
        let project_items: Vec<ListItem> = self
            .projects
            .iter()
            .map(|p| {
                let style = Style::default().fg(Color::White);
                ListItem::new(Line::from(vec![
                    Span::styled(&p.display_name, style),
                    Span::styled(
                        format!(" ({})", p.session_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();

        let projects_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Projects ({})", self.projects.len()))
            .border_style(if self.focus == Focus::Projects {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let projects_list = List::new(project_items)
            .block(projects_block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ");

        f.render_stateful_widget(projects_list, main_chunks[0], &mut self.project_state);

        // Preview pane (prominent, top right)
        let preview_text = if let Some(session) = self.selected_session() {
            let branch_info = session
                .git_branch
                .as_ref()
                .map(|b| format!(" [{}]", b))
                .unwrap_or_default();

            format!(
                "{}{}\n{} messages | {}\n\n{}",
                session.display_name(),
                branch_info,
                session.message_count,
                session.duration_str(),
                session.preview_text
            )
        } else {
            "No session selected\n\nSelect a session to see preview".to_string()
        };

        let preview = Paragraph::new(preview_text)
            .block(Block::default().borders(Borders::ALL).title("Preview"))
            .style(Style::default().fg(Color::Gray))
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(preview, right_chunks[0]);

        // Sessions pane (bottom right)
        let session_items: Vec<ListItem> = self
            .current_sessions
            .iter()
            .map(|s| {
                let name = s.display_name();
                let date = s.last_message.format(&self.config.display.date_format);

                // Show agent sessions differently if configured
                let name_style = if s.is_agent {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(vec![
                    Span::styled(name, name_style),
                    Span::raw("  "),
                    Span::styled(date.to_string(), Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let project_name = self
            .selected_project()
            .map(|p| p.display_name.clone())
            .unwrap_or_default();

        let sessions_block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                "Sessions - {} ({})",
                project_name,
                self.current_sessions.len()
            ))
            .border_style(if self.focus == Focus::Sessions {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let sessions_list = List::new(session_items)
            .block(sessions_block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ");

        f.render_stateful_widget(sessions_list, right_chunks[1], &mut self.session_state);
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        // This is called from App, which expects Option<String> for status messages
        // But the trait requires no return. We'll handle this differently.
    }
}

impl BrowserScreen {
    /// Handle key event and return optional status message.
    pub async fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.focus == Focus::Sessions {
                    self.focus = Focus::Projects;
                }
                None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.focus == Focus::Projects && !self.current_sessions.is_empty() {
                    self.focus = Focus::Sessions;
                }
                None
            }
            KeyCode::Tab => {
                // Cycle focus between panes
                self.focus = match self.focus {
                    Focus::Projects => {
                        if !self.current_sessions.is_empty() {
                            Focus::Sessions
                        } else {
                            Focus::Projects
                        }
                    }
                    Focus::Sessions => Focus::Projects,
                };
                None
            }
            KeyCode::Enter => {
                match self.focus {
                    Focus::Projects => {
                        // Enter on project switches to sessions pane
                        if !self.current_sessions.is_empty() {
                            self.focus = Focus::Sessions;
                        }
                        None
                    }
                    Focus::Sessions => self.copy_resume_command(),
                }
            }
            KeyCode::Char('y') => self.copy_resume_command(),
            _ => None,
        }
    }
}
