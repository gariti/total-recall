//! Main application state and event loop.

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_garnish::{shadow::HalfShadow, GarnishableWidget};
use std::io;
use std::sync::Arc;
use std::time::Duration;

use crate::config::Config;
use crate::screens::{BrowserScreen, Screen};
use crate::services::{SessionStore, Theme};

/// Result of running the application.
#[derive(Debug)]
pub enum AppResult {
    /// User quit normally.
    Exit,
    /// User selected a session to launch.
    LaunchSession { session_id: String, project_path: String },
    /// Start a new session in the given project.
    NewSession { project_path: String },
}

/// Application state.
pub struct App {
    current_screen: AppScreen,
    should_quit: bool,
    launch_session: Option<(String, String)>,
    new_session: Option<String>,

    // Theme
    theme: Arc<Theme>,

    // Screens
    browser_screen: BrowserScreen,

    // Status bar info
    status_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Browser,
    // Search,
    // Preview,
    // Stats,
}

impl App {
    /// Create a new application instance.
    pub async fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let theme = Arc::new(Theme::load());

        // Initialize services
        let session_store = Arc::new(SessionStore::new(config.clone())?);

        // Initialize screens
        let browser_screen = BrowserScreen::new(session_store.clone(), config.clone(), theme.clone());

        Ok(Self {
            current_screen: AppScreen::Browser,
            should_quit: false,
            launch_session: None,
            new_session: None,
            theme,
            browser_screen,
            status_message: "Loading sessions...".to_string(),
        })
    }

    /// Run the application.
    pub async fn run(&mut self) -> Result<AppResult> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Load initial data
        self.load_initial_data().await;

        // Main event loop
        let result = self.event_loop(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Load initial data for all screens.
    async fn load_initial_data(&mut self) {
        if let Err(e) = self.browser_screen.load_sessions().await {
            self.status_message = format!("Failed to load sessions: {}", e);
        } else {
            let count = self.browser_screen.session_count();
            self.status_message = format!("{} sessions loaded", count);
        }
    }

    /// Main event loop.
    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<AppResult> {
        loop {
            // Draw UI
            terminal.draw(|f| self.draw(f))?;

            // Poll for events with timeout
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Global key handlers
                    match (key.modifiers, key.code) {
                        (KeyModifiers::CONTROL, KeyCode::Char('c'))
                        | (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                            self.should_quit = true;
                        }
                        (_, KeyCode::Char('q')) => {
                            self.should_quit = true;
                        }
                        // TODO: When we have multiple screens, use Ctrl+Tab or similar to switch
                        // For now, pass Tab to the current screen for pane cycling
                        _ => {
                            // Delegate to current screen
                            match self.current_screen {
                                AppScreen::Browser => {
                                    match self.browser_screen.handle_key(key).await {
                                        crate::screens::ScreenAction::None => {}
                                        crate::screens::ScreenAction::StatusMessage(msg) => {
                                            self.status_message = msg;
                                        }
                                        crate::screens::ScreenAction::LaunchSession { session_id, project_path } => {
                                            self.launch_session = Some((session_id, project_path));
                                            self.should_quit = true;
                                        }
                                        crate::screens::ScreenAction::NewSession { project_path } => {
                                            self.new_session = Some(project_path);
                                            self.should_quit = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(if let Some((session_id, project_path)) = self.launch_session.take() {
            AppResult::LaunchSession { session_id, project_path }
        } else if let Some(project_path) = self.new_session.take() {
            AppResult::NewSession { project_path }
        } else {
            AppResult::Exit
        })
    }

    /// Draw the UI.
    fn draw(&mut self, f: &mut ratatui::Frame) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::style::{Modifier, Style};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tab bar
                Constraint::Min(0),    // Main content
                Constraint::Length(1), // Status bar
            ])
            .split(f.area());

        // Tab bar with ASCII art title
        let titles: Vec<Line> = ["Browser", "Search", "Preview", "Stats"]
            .iter()
            .map(|t| Line::from(*t))
            .collect();
        let selected = match self.current_screen {
            AppScreen::Browser => 0,
        };
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("total-recall"))
            .select(selected)
            .style(Style::default().fg(self.theme.foreground))
            .highlight_style(
                Style::default()
                    .fg(self.theme.color6)
                    .add_modifier(Modifier::BOLD),
            );
        let garnished_tabs = tabs.garnish(HalfShadow::default());
        f.render_widget(garnished_tabs, chunks[0]);

        // Main content area
        match self.current_screen {
            AppScreen::Browser => self.browser_screen.draw(f, chunks[1]),
        }

        // Status bar
        let status = Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(&self.status_message, Style::default().fg(self.theme.color7)),
            Span::raw(" │ "),
            Span::styled("j/k", Style::default().fg(self.theme.color8)),
            Span::styled(" Nav", Style::default().fg(self.theme.color7)),
            Span::raw(" │ "),
            Span::styled("Enter", Style::default().fg(self.theme.color8)),
            Span::styled(" Resume", Style::default().fg(self.theme.color7)),
            Span::raw(" │ "),
            Span::styled("n", Style::default().fg(self.theme.color8)),
            Span::styled(" New", Style::default().fg(self.theme.color7)),
            Span::raw(" │ "),
            Span::styled("q", Style::default().fg(self.theme.color8)),
            Span::styled(" Quit", Style::default().fg(self.theme.color7)),
        ]));
        f.render_widget(status, chunks[2]);
    }
}
