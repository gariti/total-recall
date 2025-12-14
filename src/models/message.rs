//! Message types from Claude Code JSONL files.

use chrono::{DateTime, Utc};
use serde::Deserialize;

/// A single entry in a session JSONL file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageEntry {
    pub uuid: String,
    #[serde(default)]
    pub parent_uuid: Option<String>,
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(default)]
    pub message: Option<MessageContent>,
    #[serde(default)]
    pub is_sidechain: bool,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
}

/// Message content - can be simple text or structured with tool calls.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Simple {
        role: String,
        content: String,
    },
    Structured {
        role: String,
        content: Vec<ContentBlock>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        usage: Option<TokenUsage>,
    },
}

impl MessageContent {
    /// Get the role of this message.
    pub fn role(&self) -> &str {
        match self {
            MessageContent::Simple { role, .. } => role,
            MessageContent::Structured { role, .. } => role,
        }
    }

    /// Get the text content of this message (first text block or simple content).
    pub fn text(&self) -> String {
        match self {
            MessageContent::Simple { content, .. } => content.clone(),
            MessageContent::Structured { content, .. } => {
                for block in content {
                    if let ContentBlock::Text { text } = block {
                        return text.clone();
                    }
                }
                String::new()
            }
        }
    }

    /// Get the model used (for assistant messages).
    pub fn model(&self) -> Option<&str> {
        match self {
            MessageContent::Simple { .. } => None,
            MessageContent::Structured { model, .. } => model.as_deref(),
        }
    }
}

/// Content block within a structured message.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
        #[serde(default)]
        is_error: bool,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(other)]
    Unknown,
}

/// Token usage information.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
}

/// Assistant message content.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantContent {
    pub role: String,
    pub content: Vec<ContentBlock>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub usage: Option<TokenUsage>,
}
