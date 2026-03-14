//! Main application state and event loop.

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
// ratatui_garnish available if needed for overlays
#[allow(unused_imports)]
use ratatui_garnish::GarnishableWidget;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Config;
use crate::event::{self, AppEvent};
use crate::screens::{
    AgentDetailScreen, BrowserScreen, DashboardScreen, Screen, ScreenAction, SpawnWizard,
};
use crate::services::{AgentManager, SessionStore, Theme};

/// Result of running the application.
#[derive(Debug)]
pub enum AppResult {
    /// User quit normally.
    Exit,
    /// User selected a session to launch.
    LaunchSession { session_id: String, project_path: String },
    /// Start a new session in the given project.
    NewSession { project_path: String },
    /// Open lazygit in the project directory.
    OpenLazygit { project_path: String },
    /// Open GitHub in browser.
    OpenGithub { project_path: String },
    /// Open terminal in the project directory.
    OpenTerminal { project_path: String },
    /// Open editor in the project directory.
    OpenEditor { project_path: String },
}

/// Which tab/screen is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    /// Mission control — agent overview.
    Dashboard,
    /// Focused view of one agent's output.
    AgentDetail,
    /// Session browser (the original total-recall).
    Sessions,
}

/// Application state.
pub struct App {
    current_screen: AppScreen,
    should_quit: bool,
    launch_session: Option<(String, String)>,
    new_session: Option<String>,
    open_lazygit: Option<String>,
    open_github: Option<String>,
    open_terminal: Option<String>,
    open_editor: Option<String>,

    // Theme
    theme: Arc<Theme>,
    config: Arc<Config>,

    // Services
    agent_manager: AgentManager,

    // Screens
    dashboard_screen: DashboardScreen,
    agent_detail_screen: AgentDetailScreen,
    browser_screen: BrowserScreen,
    spawn_wizard: SpawnWizard,

    // Agent detail: which agent index we're viewing
    focused_agent_index: Option<usize>,

    // Status bar info
    status_message: String,

    // Tick counter for periodic agent polling
    tick_count: u64,
}

impl App {
    /// Create a new application instance.
    pub async fn new(config: Config) -> Result<Self> {
        let config = Arc::new(config);
        let theme = Arc::new(Theme::load());

        // Initialize services
        let session_store = Arc::new(SessionStore::new(config.clone())?);

        // We'll create a temporary sender for AgentManager construction.
        // The real event loop sender replaces this later.
        let (event_tx, _) = tokio::sync::mpsc::unbounded_channel();
        let agent_manager = AgentManager::new(config.clone(), event_tx)?;

        // Initialize screens
        let browser_screen = BrowserScreen::new(session_store.clone(), config.clone(), theme.clone());
        let dashboard_screen = DashboardScreen::new(theme.clone());
        let agent_detail_screen = AgentDetailScreen::new(theme.clone());
        let spawn_wizard = SpawnWizard::new(theme.clone());

        Ok(Self {
            current_screen: AppScreen::Dashboard,
            should_quit: false,
            launch_session: None,
            new_session: None,
            open_lazygit: None,
            open_github: None,
            open_terminal: None,
            open_editor: None,
            theme,
            config,
            agent_manager,
            dashboard_screen,
            agent_detail_screen,
            browser_screen,
            spawn_wizard,
            focused_agent_index: None,
            status_message: "Loading...".to_string(),
            tick_count: 0,
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
            let agent_count = self.agent_manager.active_count();
            self.status_message = format!(
                "{} sessions | {} active agents",
                count, agent_count
            );
        }

        // Reconcile persisted agents with actual tmux state
        self.agent_manager.poll_agents();
    }

    /// Main event loop using channel-based events.
    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<AppResult> {
        let (event_tx, mut event_rx) = event::spawn_event_tasks();

        // Give the agent manager the real event sender
        // (We can't swap it in after construction easily, but the tick-based polling
        // in handle_tick() handles agent monitoring directly)
        let _ = event_tx; // keep alive to prevent input thread from stopping

        loop {
            // Draw UI
            terminal.draw(|f| self.draw(f))?;

            // Wait for next event
            if let Some(ev) = event_rx.recv().await {
                match ev {
                    AppEvent::Input(key) => {
                        tracing::debug!("Key received: {:?} (screen: {:?})", key.code, self.current_screen);
                        // Spawn wizard gets priority if active
                        if self.spawn_wizard.active {
                            if let Some(request) = self.spawn_wizard.handle_key(key) {
                                self.handle_spawn_request(request);
                            }
                            continue;
                        }

                        // Global key handlers
                        match (key.modifiers, key.code) {
                            (KeyModifiers::CONTROL, KeyCode::Char('c'))
                            | (KeyModifiers::CONTROL, KeyCode::Char('q')) => {
                                self.should_quit = true;
                            }
                            // Tab toggles between Dashboard and Sessions
                            (KeyModifiers::NONE, KeyCode::Tab) => {
                                self.current_screen = match self.current_screen {
                                    AppScreen::Dashboard => AppScreen::Sessions,
                                    AppScreen::Sessions => AppScreen::Dashboard,
                                    AppScreen::AgentDetail => AppScreen::Dashboard,
                                };
                            }
                            // Esc from AgentDetail goes back to Dashboard
                            (KeyModifiers::NONE, KeyCode::Esc) if self.current_screen == AppScreen::AgentDetail => {
                                self.current_screen = AppScreen::Dashboard;
                                self.focused_agent_index = None;
                            }
                            // Esc quits from sessions screen
                            (KeyModifiers::NONE, KeyCode::Esc) if self.current_screen == AppScreen::Sessions => {
                                self.should_quit = true;
                            }
                            _ => {
                                // Delegate to current screen
                                let action = self.handle_screen_key(key).await;
                                self.process_action(action);
                            }
                        }
                    }
                    AppEvent::Tick => {
                        self.handle_tick();
                    }
                    AppEvent::AgentUpdate { agent_id } | AppEvent::AgentExited { agent_id } => {
                        // Agent events update status message
                        if let Some(agent) = self.agent_manager.get_by_id_mut(&agent_id) {
                            self.status_message = format!(
                                "Agent {} — {}",
                                agent.name,
                                agent.status.label()
                            );
                        }
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }

        Ok(self.build_result())
    }

    /// Handle key events for the current screen.
    async fn handle_screen_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenAction {
        match self.current_screen {
            AppScreen::Dashboard => self.handle_dashboard_key(key),
            AppScreen::AgentDetail => self.handle_detail_key(key),
            AppScreen::Sessions => self.browser_screen.handle_key(key).await,
        }
    }

    /// Handle keys on the dashboard screen — DOS game style, no letters.
    fn handle_dashboard_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenAction {
        let agent_count = self.agent_manager.agents().len();
        self.dashboard_screen.clamp_selection(agent_count);

        match key.code {
            KeyCode::Left => {
                self.dashboard_screen.move_left();
                ScreenAction::None
            }
            KeyCode::Right => {
                self.dashboard_screen.move_right(agent_count);
                ScreenAction::None
            }
            KeyCode::Enter => {
                if agent_count > 0 {
                    ScreenAction::FocusAgent { index: self.dashboard_screen.selected }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Insert | KeyCode::Char('n') => ScreenAction::OpenSpawnWizard,
            KeyCode::Delete => {
                tracing::debug!("Del pressed, agent_count={}, selected={}", agent_count, self.dashboard_screen.selected);
                if agent_count > 0 {
                    ScreenAction::KillAgent { index: self.dashboard_screen.selected }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Backspace => {
                if agent_count > 0 {
                    ScreenAction::DeleteAgent { index: self.dashboard_screen.selected }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::F(1) => {
                if agent_count > 0 {
                    ScreenAction::AttachAgent { index: self.dashboard_screen.selected }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::F(5) => {
                self.agent_manager.poll_agents();
                ScreenAction::StatusMessage("Refreshed".to_string())
            }
            KeyCode::Esc => {
                self.should_quit = true;
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    /// Handle keys on the agent detail screen — DOS game style.
    fn handle_detail_key(&mut self, key: crossterm::event::KeyEvent) -> ScreenAction {
        match key.code {
            KeyCode::Esc => ScreenAction::BackToDashboard,
            KeyCode::F(1) => {
                if let Some(idx) = self.focused_agent_index {
                    ScreenAction::AttachAgent { index: idx }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Delete => {
                if let Some(idx) = self.focused_agent_index {
                    ScreenAction::KillAgent { index: idx }
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Down => {
                if let Some(idx) = self.focused_agent_index {
                    let max_lines = self.agent_manager.get(idx)
                        .map(|a| a.last_output_lines.len())
                        .unwrap_or(0);
                    self.agent_detail_screen.scroll_down(max_lines);
                }
                ScreenAction::None
            }
            KeyCode::Up => {
                self.agent_detail_screen.scroll_up();
                ScreenAction::None
            }
            KeyCode::End => {
                self.agent_detail_screen.scroll_to_bottom();
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    /// Process a screen action.
    fn process_action(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::StatusMessage(msg) => {
                self.status_message = msg;
            }
            ScreenAction::LaunchSession { session_id, project_path } => {
                self.launch_session = Some((session_id, project_path));
                self.should_quit = true;
            }
            ScreenAction::NewSession { project_path } => {
                self.new_session = Some(project_path);
                self.should_quit = true;
            }
            ScreenAction::OpenLazygit { project_path } => {
                self.open_lazygit = Some(project_path);
                self.should_quit = true;
            }
            ScreenAction::OpenGithub { project_path } => {
                self.open_github = Some(project_path);
                self.should_quit = true;
            }
            ScreenAction::OpenTerminal { project_path } => {
                self.open_terminal = Some(project_path);
                self.should_quit = true;
            }
            ScreenAction::OpenEditor { project_path } => {
                self.open_editor = Some(project_path);
                self.should_quit = true;
            }
            ScreenAction::OpenSpawnWizard => {
                let projects = self.browser_screen.projects().to_vec();
                let registry = self.agent_manager.registry();
                self.spawn_wizard.open(projects, registry);
            }
            ScreenAction::KillAgent { index } => {
                tracing::debug!("Processing KillAgent at index {}", index);
                // If agent is already dead, Del removes it entirely
                let already_dead = self.agent_manager.get(index)
                    .is_some_and(|a| !a.status.is_alive());
                if already_dead {
                    let name = self.agent_manager.get(index)
                        .map(|a| a.name.clone())
                        .unwrap_or_default();
                    tracing::debug!("Agent {} already dead, deleting instead", name);
                    match self.agent_manager.delete(index) {
                        Ok(()) => {
                            self.status_message = format!("Removed {}", name);
                            self.dashboard_screen.clamp_selection(self.agent_manager.agents().len());
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to remove: {}", e);
                        }
                    }
                } else {
                    match self.agent_manager.kill(index) {
                        Ok(()) => {
                            if let Some(agent) = self.agent_manager.get(index) {
                                tracing::debug!("Kill succeeded, status now {:?}", agent.status);
                                self.status_message = format!("Killed agent {}", agent.name);
                            }
                        }
                        Err(e) => {
                            tracing::debug!("Kill failed: {}", e);
                            self.status_message = format!("Failed to kill: {}", e);
                        }
                    }
                }
            }
            ScreenAction::DeleteAgent { index } => {
                let name = self.agent_manager.get(index)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                match self.agent_manager.delete(index) {
                    Ok(()) => {
                        self.status_message = format!("Deleted agent {}", name);
                        self.dashboard_screen.clamp_selection(self.agent_manager.agents().len());
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to delete agent: {}", e);
                    }
                }
            }
            ScreenAction::FocusAgent { index } => {
                if index < self.agent_manager.agents().len() {
                    self.focused_agent_index = Some(index);
                    self.agent_detail_screen.reset_scroll();
                    self.current_screen = AppScreen::AgentDetail;
                }
            }
            ScreenAction::AttachAgent { index } => {
                if let Some(agent) = self.agent_manager.get(index) {
                    if agent.status.is_alive() {
                        let tmux_session = agent.tmux_session.clone();
                        let _ = std::process::Command::new("ghostty")
                            .arg("-e")
                            .arg("tmux")
                            .arg("attach-session")
                            .arg("-t")
                            .arg(&tmux_session)
                            .spawn();
                        self.status_message = format!("Attached to {}", agent.name);
                    } else {
                        self.status_message = format!("Agent {} is not alive", agent.name);
                    }
                }
            }
            ScreenAction::BackToDashboard => {
                self.focused_agent_index = None;
                self.current_screen = AppScreen::Dashboard;
            }
        }
    }

    /// Handle spawn request from the wizard.
    fn handle_spawn_request(&mut self, request: crate::screens::spawn_wizard::SpawnRequest) {
        let project_path = PathBuf::from(&request.project_path);
        match self.agent_manager.spawn(
            project_path,
            request.agent_type,
            request.task_prompt,
            request.use_worktree,
        ) {
            Ok(index) => {
                if let Some(agent) = self.agent_manager.get(index) {
                    self.status_message = format!("Spawned agent {}", agent.name);
                }
                self.current_screen = AppScreen::Dashboard;
                self.dashboard_screen.selected = index;
            }
            Err(e) => {
                self.status_message = format!("Failed to spawn agent: {}", e);
            }
        }
    }

    /// Periodic tick handler — ticks dashboard every tick, polls agents every 6th.
    fn handle_tick(&mut self) {
        self.tick_count += 1;

        // Tick dashboard animations every tick
        self.dashboard_screen.tick();

        // Poll agents every 6 ticks (3 seconds at 500ms tick rate)
        if self.tick_count % 6 == 0 {
            self.agent_manager.poll_agents();

            // Update status
            let active = self.agent_manager.active_count();
            let total = self.agent_manager.agents().len();
            if total > 0 {
                self.status_message = format!(
                    "{} agents ({} active)",
                    total, active
                );
            }
        }
    }

    /// Build the final AppResult from state flags.
    fn build_result(&mut self) -> AppResult {
        if let Some((session_id, project_path)) = self.launch_session.take() {
            AppResult::LaunchSession { session_id, project_path }
        } else if let Some(project_path) = self.new_session.take() {
            AppResult::NewSession { project_path }
        } else if let Some(project_path) = self.open_lazygit.take() {
            AppResult::OpenLazygit { project_path }
        } else if let Some(project_path) = self.open_github.take() {
            AppResult::OpenGithub { project_path }
        } else if let Some(project_path) = self.open_terminal.take() {
            AppResult::OpenTerminal { project_path }
        } else if let Some(project_path) = self.open_editor.take() {
            AppResult::OpenEditor { project_path }
        } else {
            AppResult::Exit
        }
    }

    /// Draw the UI — full-screen, no tab bar.
    fn draw(&mut self, f: &mut ratatui::Frame) {
        let area = f.area();

        // Main content area — full screen
        match self.current_screen {
            AppScreen::Dashboard => {
                let agents = self.agent_manager.agents();
                let active = self.agent_manager.active_count();
                self.dashboard_screen.draw(f, area, agents, active, &self.status_message);
            }
            AppScreen::AgentDetail => {
                if let Some(idx) = self.focused_agent_index {
                    if let Some(agent) = self.agent_manager.get(idx) {
                        let agent_clone = agent.clone();
                        self.agent_detail_screen.draw(f, area, &agent_clone);
                    }
                }
            }
            AppScreen::Sessions => {
                self.browser_screen.draw(f, area);
            }
        }

        // Spawn wizard overlay (drawn on top)
        if self.spawn_wizard.active {
            self.spawn_wizard.draw(f, f.area());
        }
    }
}
