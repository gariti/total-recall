//! Browser screen - main interface for browsing projects and sessions.

use async_trait::async_trait;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use ratatui_garnish::{
    shadow::HalfShadow, GarnishableStatefulWidget, GarnishableWidget, Padding,
};
use std::sync::Arc;

use crate::config::Config;
use crate::models::{Project, Session};
use crate::services::{ascii_art, SessionStore, Theme};

use super::{Screen, ScreenAction};

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
    theme: Arc<Theme>,

    // UI state
    focus: Focus,
    project_state: ListState,
    session_state: ListState,
    sessions_visible: bool,

    // Cached data
    projects: Vec<Project>,
    current_sessions: Vec<Session>,

    // Splash art (randomly selected on startup)
    splash_art: &'static str,
}

impl BrowserScreen {
    /// Create a new browser screen.
    pub fn new(session_store: Arc<SessionStore>, config: Arc<Config>, theme: Arc<Theme>) -> Self {
        let mut project_state = ListState::default();
        project_state.select(Some(0));

        Self {
            session_store,
            config,
            theme,
            focus: Focus::Projects,
            project_state,
            session_state: ListState::default(),
            sessions_visible: false,
            projects: Vec::new(),
            current_sessions: Vec::new(),
            splash_art: ascii_art::random_art(),
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

        // Split right column: Preview on top (prominent), Sessions below (only if visible)
        let right_chunks = if self.sessions_visible {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(60), // Preview (prominent)
                    Constraint::Percentage(40), // Sessions list
                ])
                .split(main_chunks[1])
        } else {
            // Sessions hidden - preview takes full right side
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)])
                .split(main_chunks[1])
        };

        // Projects pane
        let project_items: Vec<ListItem> = self
            .projects
            .iter()
            .map(|p| {
                let style = Style::default().fg(self.theme.foreground);
                ListItem::new(Line::from(vec![
                    Span::styled(&p.display_name, style),
                    Span::styled(
                        format!(" ({})", p.session_count),
                        Style::default().fg(self.theme.color8),
                    ),
                ]))
            })
            .collect();

        let projects_block = Block::default()
            .borders(Borders::ALL)
            .title(format!("Projects ({})", self.projects.len()))
            .border_style(if self.focus == Focus::Projects {
                Style::default().fg(self.theme.color6)
            } else {
                Style::default().fg(self.theme.color8)
            });

        let projects_list = List::new(project_items)
            .block(projects_block)
            .highlight_style(
                Style::default()
                    .bg(self.theme.color8)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ");

        // Add shadow effect when focused
        if self.focus == Focus::Projects {
            let garnished =
                GarnishableStatefulWidget::garnish(projects_list, HalfShadow::default());
            f.render_stateful_widget(garnished, main_chunks[0], &mut self.project_state);
        } else {
            f.render_stateful_widget(projects_list, main_chunks[0], &mut self.project_state);
        }

        // Preview pane (prominent, top right)
        // Show ASCII art when on Projects view, show session preview when on Sessions view
        let (preview_title, preview_text) = if self.focus == Focus::Projects {
            ("total-recall", self.splash_art.to_string())
        } else if let Some(session) = self.selected_session() {
            let branch_info = session
                .git_branch
                .as_ref()
                .map(|b| format!(" [{}]", b))
                .unwrap_or_default();

            (
                "Preview",
                format!(
                    "{}{}\n{} messages | {}\n\n{}",
                    session.display_name(),
                    branch_info,
                    session.message_count,
                    session.duration_str(),
                    session.preview_text
                ),
            )
        } else {
            (
                "Preview",
                "No session selected\n\nSelect a session to see preview".to_string(),
            )
        };

        let preview_block = Block::default()
            .borders(Borders::ALL)
            .title(preview_title);

        // Don't wrap ASCII art, only wrap preview text
        if self.focus == Focus::Projects {
            let preview = Paragraph::new(preview_text)
                .block(preview_block)
                .style(Style::default().fg(self.theme.color6));

            let garnished_preview = preview
                .garnish(Padding::horizontal(1))
                .garnish(HalfShadow::default());
            f.render_widget(garnished_preview, right_chunks[0]);
        } else {
            let preview = Paragraph::new(preview_text)
                .block(preview_block)
                .style(Style::default().fg(self.theme.color7))
                .wrap(ratatui::widgets::Wrap { trim: true });

            let garnished_preview = preview
                .garnish(Padding::horizontal(1))
                .garnish(HalfShadow::default());
            f.render_widget(garnished_preview, right_chunks[0]);
        }

        // Sessions pane (bottom right) - only show when visible
        if self.sessions_visible {
            let session_items: Vec<ListItem> = self
                .current_sessions
                .iter()
                .map(|s| {
                    let date = s.last_message.format(&self.config.display.date_format);

                    // Use preview text instead of session name
                    let preview = if s.preview_text.is_empty() {
                        s.display_name()
                    } else {
                        // Truncate to fit in list
                        if s.preview_text.len() > 60 {
                            format!("{}...", &s.preview_text[..60])
                        } else {
                            s.preview_text.clone()
                        }
                    };

                    // Show agent sessions differently
                    let text_style = if s.is_agent {
                        Style::default().fg(self.theme.color5)
                    } else {
                        Style::default().fg(self.theme.foreground)
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(date.to_string(), Style::default().fg(self.theme.color8)),
                        Span::raw("  "),
                        Span::styled(preview, text_style),
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
                    Style::default().fg(self.theme.color6)
                } else {
                    Style::default().fg(self.theme.color8)
                });

            let sessions_list = List::new(session_items)
                .block(sessions_block)
                .highlight_style(
                    Style::default()
                        .bg(self.theme.color8)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("► ");

            // Add shadow effect when focused
            if self.focus == Focus::Sessions {
                let garnished =
                    GarnishableStatefulWidget::garnish(sessions_list, HalfShadow::default());
                f.render_stateful_widget(garnished, right_chunks[1], &mut self.session_state);
            } else {
                f.render_stateful_widget(sessions_list, right_chunks[1], &mut self.session_state);
            }
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        // This is called from App, which expects Option<String> for status messages
        // But the trait requires no return. We'll handle this differently.
    }
}

impl BrowserScreen {
    /// Handle key event and return action.
    pub async fn handle_key(&mut self, key: KeyEvent) -> ScreenAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                ScreenAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                ScreenAction::None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.focus == Focus::Sessions {
                    self.focus = Focus::Projects;
                    self.sessions_visible = false;
                }
                ScreenAction::None
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.focus == Focus::Projects && !self.current_sessions.is_empty() {
                    self.sessions_visible = true;
                    self.focus = Focus::Sessions;
                }
                ScreenAction::None
            }
            KeyCode::Tab => {
                // Cycle focus between panes
                match self.focus {
                    Focus::Projects => {
                        if !self.current_sessions.is_empty() {
                            self.sessions_visible = true;
                            self.focus = Focus::Sessions;
                        }
                    }
                    Focus::Sessions => {
                        self.sessions_visible = false;
                        self.focus = Focus::Projects;
                    }
                };
                ScreenAction::None
            }
            KeyCode::Enter | KeyCode::Char('y') => {
                match self.focus {
                    Focus::Projects => {
                        // Enter on project shows sessions and switches to sessions pane
                        if !self.current_sessions.is_empty() {
                            self.sessions_visible = true;
                            self.focus = Focus::Sessions;
                        }
                        ScreenAction::None
                    }
                    Focus::Sessions => {
                        if let Some(session) = self.selected_session() {
                            ScreenAction::LaunchSession {
                                session_id: session.id.clone(),
                                project_path: session.project_path.clone(),
                            }
                        } else {
                            ScreenAction::None
                        }
                    }
                }
            }
            KeyCode::Char('n') => {
                // Start a new session in the selected project
                if let Some(project) = self.selected_project() {
                    ScreenAction::NewSession {
                        project_path: project.decoded_path.clone(),
                    }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('g') => {
                // Open lazygit in the project directory
                if let Some(project) = self.selected_project() {
                    ScreenAction::OpenLazygit {
                        project_path: project.decoded_path.clone(),
                    }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('b') => {
                // Open GitHub in browser
                if let Some(project) = self.selected_project() {
                    ScreenAction::OpenGithub {
                        project_path: project.decoded_path.clone(),
                    }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('t') => {
                // Open terminal in the project directory
                if let Some(project) = self.selected_project() {
                    ScreenAction::OpenTerminal {
                        project_path: project.decoded_path.clone(),
                    }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('e') => {
                // Open editor in the project directory
                if let Some(project) = self.selected_project() {
                    ScreenAction::OpenEditor {
                        project_path: project.decoded_path.clone(),
                    }
                } else {
                    ScreenAction::None
                }
            }
            _ => ScreenAction::None,
        }
    }
}
