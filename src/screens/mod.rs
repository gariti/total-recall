//! TUI screens.

pub mod agent_detail;
pub mod browser;
pub mod dashboard;
pub mod spawn_wizard;

pub use agent_detail::AgentDetailScreen;
pub use browser::BrowserScreen;
pub use dashboard::DashboardScreen;
pub use spawn_wizard::SpawnWizard;

use async_trait::async_trait;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

/// Action returned by screen key handlers.
#[derive(Debug)]
pub enum ScreenAction {
    /// No action needed.
    None,
    /// Display a status message.
    StatusMessage(String),
    /// Launch a Claude session with the given ID and project path.
    LaunchSession { session_id: String, project_path: String },
    /// Start a new Claude session in the given project path.
    NewSession { project_path: String },
    /// Open lazygit in the project directory.
    OpenLazygit { project_path: String },
    /// Open GitHub in browser.
    OpenGithub { project_path: String },
    /// Open terminal in the project directory.
    OpenTerminal { project_path: String },
    /// Open editor in the project directory.
    OpenEditor { project_path: String },
    /// Open the spawn wizard.
    OpenSpawnWizard,
    /// Kill an agent by index.
    KillAgent { index: usize },
    /// Delete an agent by index.
    DeleteAgent { index: usize },
    /// Focus on an agent (switch to detail view).
    FocusAgent { index: usize },
    /// Attach to an agent's tmux session in a new terminal.
    AttachAgent { index: usize },
    /// Go back from detail to dashboard.
    BackToDashboard,
}

/// Trait for screen implementations.
#[async_trait]
pub trait Screen {
    /// Draw the screen.
    fn draw(&mut self, f: &mut Frame, area: Rect);

    /// Handle a key event.
    async fn handle_key(&mut self, key: KeyEvent);
}
