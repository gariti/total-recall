//! Configuration management.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub claude: ClaudeConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            claude: ClaudeConfig::default(),
            display: DisplayConfig::default(),
        }
    }
}

/// Claude-related configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    /// Path to Claude directory (default: ~/.claude)
    #[serde(default = "default_claude_dir")]
    pub claude_dir: String,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            claude_dir: default_claude_dir(),
        }
    }
}

fn default_claude_dir() -> String {
    dirs::home_dir()
        .map(|h| h.join(".claude").to_string_lossy().to_string())
        .unwrap_or_else(|| "~/.claude".to_string())
}

/// Display configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Number of preview lines to show
    #[serde(default = "default_preview_lines")]
    pub preview_lines: usize,
    /// Date format string
    #[serde(default = "default_date_format")]
    pub date_format: String,
    /// Show agent sessions (sidechains)
    #[serde(default = "default_show_agents")]
    pub show_agent_sessions: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            preview_lines: default_preview_lines(),
            date_format: default_date_format(),
            show_agent_sessions: default_show_agents(),
        }
    }
}

fn default_preview_lines() -> usize {
    3
}

fn default_date_format() -> String {
    "%m/%d %H:%M".to_string()
}

fn default_show_agents() -> bool {
    true
}

impl Config {
    /// Load configuration from default location.
    pub fn load() -> Result<Self> {
        let config_path = Self::default_config_path();

        if config_path.exists() {
            Self::from_file(&config_path.to_string_lossy())
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific file.
    pub fn from_file(path: &str) -> Result<Self> {
        let expanded = expand_path(path);
        let content = std::fs::read_to_string(&expanded)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Get the default config path.
    pub fn default_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("total-recall")
            .join("config.toml")
    }

    /// Get the data directory for metadata storage.
    pub fn data_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("total-recall")
    }

    /// Get the Claude projects directory.
    pub fn claude_projects_dir(&self) -> PathBuf {
        let claude_dir = expand_path(&self.claude.claude_dir);
        PathBuf::from(claude_dir).join("projects")
    }
}

/// Expand ~ to home directory.
fn expand_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
