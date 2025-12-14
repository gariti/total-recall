//! Session summary data.

use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Summary of a Claude Code session.
#[derive(Debug, Clone)]
pub struct Session {
    /// Session UUID
    pub id: String,
    /// Decoded project path (e.g., "/home/user/projects/myapp")
    pub project_path: String,
    /// Human-readable session slug (e.g., "twinkly-singing-nova")
    pub slug: Option<String>,
    /// Git branch at session start
    pub git_branch: Option<String>,
    /// Timestamp of first message
    pub first_message: DateTime<Utc>,
    /// Timestamp of last message
    pub last_message: DateTime<Utc>,
    /// Total message count
    pub message_count: usize,
    /// First user message (truncated for preview)
    pub preview_text: String,
    /// Path to the JSONL session file
    pub file_path: PathBuf,
    /// File size in bytes
    pub file_size: u64,
    /// Whether this is a sidechain/agent session
    pub is_agent: bool,
    /// Agent ID if this is an agent session
    pub agent_id: Option<String>,
}

impl Session {
    /// Get a display name for the session.
    pub fn display_name(&self) -> String {
        if let Some(slug) = &self.slug {
            slug.clone()
        } else if let Some(agent_id) = &self.agent_id {
            format!("agent-{}", agent_id)
        } else {
            // Fall back to shortened session ID
            self.id.chars().take(8).collect()
        }
    }

    /// Get the resume command for this session.
    pub fn resume_command(&self) -> String {
        format!("claude --resume {}", self.id)
    }

    /// Calculate approximate duration.
    pub fn duration(&self) -> chrono::Duration {
        self.last_message.signed_duration_since(self.first_message)
    }

    /// Format duration as human-readable string.
    pub fn duration_str(&self) -> String {
        let dur = self.duration();
        let hours = dur.num_hours();
        let minutes = dur.num_minutes() % 60;

        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m", minutes)
        } else {
            "< 1m".to_string()
        }
    }
}
