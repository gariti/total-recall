//! TUI screens.

pub mod browser;
// pub mod preview;
// pub mod search;
// pub mod stats;

pub use browser::BrowserScreen;

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
}

/// Trait for screen implementations.
#[async_trait]
pub trait Screen {
    /// Draw the screen.
    fn draw(&mut self, f: &mut Frame, area: Rect);

    /// Handle a key event.
    async fn handle_key(&mut self, key: KeyEvent);
}
