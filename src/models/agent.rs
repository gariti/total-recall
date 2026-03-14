//! Agent model — represents a spawned Claude Code agent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Status of an agent's lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    /// tmux created, claude booting up
    Starting,
    /// JSONL growing (messages in last 30s)
    Active,
    /// No JSONL activity for >30s
    Idle,
    /// Process exited cleanly
    Complete,
    /// Process died unexpectedly
    Failed,
    /// User killed it
    Killed,
}

impl AgentStatus {
    /// Single-character status indicator.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Starting => "~",
            Self::Active => "●",
            Self::Idle => "○",
            Self::Complete => "◆",
            Self::Failed => "✗",
            Self::Killed => "✗",
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Starting => "STARTING",
            Self::Active => "ACTIVE",
            Self::Idle => "IDLE",
            Self::Complete => "DONE",
            Self::Failed => "FAILED",
            Self::Killed => "KILLED",
        }
    }

    /// Whether the agent is still running (tmux session alive).
    pub fn is_alive(&self) -> bool {
        matches!(self, Self::Starting | Self::Active | Self::Idle)
    }
}

/// A spawned Claude Code agent tracked by total-recall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Human-readable name (auto-generated or user-chosen).
    pub name: String,
    /// Agent type key from agent-registry.json (e.g., "nixos-colorist").
    pub agent_type: String,
    /// The project this agent is working on.
    pub project_path: PathBuf,
    /// Optional jj worktree path (e.g., /tmp/tr-riri-blur-fix/).
    pub worktree_path: Option<PathBuf>,
    /// tmux session name.
    pub tmux_session: String,
    /// Claude session ID discovered from JSONL after spawn.
    pub claude_session_id: Option<String>,
    /// Current lifecycle status.
    pub status: AgentStatus,
    /// The task prompt given to claude.
    pub task_prompt: String,
    /// When the agent was spawned.
    pub spawned_at: DateTime<Utc>,
    /// Last detected activity timestamp.
    pub last_activity: DateTime<Utc>,
    /// Number of messages seen in JSONL.
    pub message_count: usize,
    /// Last tool the agent used (from JSONL tool_use entries).
    pub last_tool: Option<String>,
    /// Last N lines captured from the tmux pane.
    pub last_output_lines: Vec<String>,
}

impl Agent {
    /// Time since last activity as a human-readable string.
    pub fn time_since_activity(&self) -> String {
        let now = Utc::now();
        let delta = now - self.last_activity;
        let secs = delta.num_seconds();

        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else {
            format!("{}h", secs / 3600)
        }
    }

    /// Activity level as a 0-10 score based on messages per minute.
    /// Used to render the activity bar.
    pub fn activity_level(&self) -> u8 {
        if !self.status.is_alive() {
            return if self.status == AgentStatus::Complete { 10 } else { 0 };
        }
        let elapsed_mins = (Utc::now() - self.spawned_at).num_seconds() as f64 / 60.0;
        if elapsed_mins < 0.1 {
            return 1; // just started
        }
        let rate = self.message_count as f64 / elapsed_mins;
        // Scale: 0 msg/min = 0, 5+ msg/min = 10
        (rate * 2.0).min(10.0) as u8
    }
}
