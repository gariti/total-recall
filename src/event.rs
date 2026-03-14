//! Channel-based event system.
//!
//! Replaces the old `event::poll` loop with an mpsc channel that aggregates
//! keyboard input, agent status updates, and periodic ticks into a single stream.

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use std::time::Duration;
use tokio::sync::mpsc;

/// Events that the application reacts to.
#[derive(Debug)]
pub enum AppEvent {
    /// Keyboard/mouse input from crossterm.
    Input(KeyEvent),
    /// An agent's status or output changed.
    AgentUpdate { agent_id: String },
    /// An agent's process exited.
    AgentExited { agent_id: String },
    /// Periodic tick for animations, clocks, activity checks.
    Tick,
}

/// Spawns background tasks that feed events into the returned receiver.
///
/// - A crossterm input reader on a blocking thread
/// - A tick timer (500ms interval)
///
/// Agent monitoring tasks push events through the sender clone they receive.
pub fn spawn_event_tasks() -> (mpsc::UnboundedSender<AppEvent>, mpsc::UnboundedReceiver<AppEvent>) {
    let (tx, rx) = mpsc::unbounded_channel();

    // Crossterm input reader (must run on a blocking thread)
    let input_tx = tx.clone();
    std::thread::spawn(move || {
        loop {
            // Poll with a short timeout so the thread can exit when the channel closes
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    // Only forward key-press events; ignore release/repeat
                    // (Kitty keyboard protocol sends all three)
                    if key.kind == KeyEventKind::Press {
                        if input_tx.send(AppEvent::Input(key)).is_err() {
                            break; // receiver dropped, app is shutting down
                        }
                    }
                }
            }
        }
    });

    // Tick timer
    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    (tx, rx)
}
