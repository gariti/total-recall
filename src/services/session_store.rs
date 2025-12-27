//! Session store - discovers and parses Claude Code sessions.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Arc;
use walkdir::WalkDir;

use crate::config::Config;
use crate::models::project::{decode_project_path, Project};
use crate::models::{MessageEntry, Session};

/// Service for discovering and loading Claude Code sessions.
pub struct SessionStore {
    config: Arc<Config>,
    /// Cached projects
    projects: Vec<Project>,
    /// Cached sessions by project encoded path
    sessions: HashMap<String, Vec<Session>>,
}

impl SessionStore {
    /// Create a new session store.
    pub fn new(config: Arc<Config>) -> Result<Self> {
        Ok(Self {
            config,
            projects: Vec::new(),
            sessions: HashMap::new(),
        })
    }

    /// Scan for all projects and their sessions.
    pub fn scan(&mut self) -> Result<()> {
        let projects_dir = self.config.claude_projects_dir();

        if !projects_dir.exists() {
            return Ok(());
        }

        let mut projects = Vec::new();

        // Iterate over project directories
        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let encoded_path = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if encoded_path.is_empty() {
                continue;
            }

            // Load sessions for this project
            let sessions = self.load_project_sessions(&path)?;

            if sessions.is_empty() {
                continue;
            }

            // Create project summary
            let mut project = Project::new(encoded_path.clone());
            project.session_count = sessions.len();
            project.total_messages = sessions.iter().map(|s| s.message_count).sum();
            project.last_activity = sessions
                .iter()
                .map(|s| s.last_message)
                .max()
                .unwrap_or(chrono::DateTime::<chrono::Utc>::MIN_UTC);

            self.sessions.insert(encoded_path, sessions);
            projects.push(project);
        }

        // Sort projects by last activity (most recent first)
        projects.sort_by(|a, b| b.last_activity.cmp(&a.last_activity));
        self.projects = projects;

        Ok(())
    }

    /// Load sessions for a specific project directory.
    fn load_project_sessions(&self, project_dir: &PathBuf) -> Result<Vec<Session>> {
        let mut sessions = Vec::new();

        for entry in WalkDir::new(project_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Only process .jsonl files
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            if let Ok(session) = self.parse_session_summary(path.to_path_buf()) {
                // Skip agent/sidechain sessions - they can't be resumed independently
                if !session.is_agent {
                    sessions.push(session);
                }
            }
        }

        // Sort sessions by last message (most recent first)
        sessions.sort_by(|a, b| b.last_message.cmp(&a.last_message));

        Ok(sessions)
    }

    /// Parse a session JSONL file and extract summary information.
    fn parse_session_summary(&self, file_path: PathBuf) -> Result<Session> {
        let file = File::open(&file_path)?;
        let metadata = file.metadata()?;
        let reader = BufReader::new(file);

        // Always use filename as session ID - this is what Claude uses to find sessions.
        // The sessionId field in entries can differ (e.g., agent sessions have parent's ID).
        let session_id = file_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let mut project_path = String::new();
        let mut slug: Option<String> = None;
        let mut git_branch: Option<String> = None;
        let mut first_message: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut last_message: Option<chrono::DateTime<chrono::Utc>> = None;
        let mut message_count = 0;
        let mut preview_text = String::new();
        let mut is_agent = false;
        let mut agent_id: Option<String> = None;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<MessageEntry>(&line) {
                message_count += 1;

                // Get session metadata from first entry
                if project_path.is_empty() {
                    if let Some(cwd) = &entry.cwd {
                        project_path = cwd.clone();
                    }
                }
                if slug.is_none() {
                    slug = entry.slug.clone();
                }
                if git_branch.is_none() {
                    git_branch = entry.git_branch.clone();
                }
                if agent_id.is_none() {
                    agent_id = entry.agent_id.clone();
                }

                // Track agent status
                if entry.is_sidechain {
                    is_agent = true;
                }

                // Track timestamps
                if first_message.is_none() {
                    first_message = Some(entry.timestamp);
                }
                last_message = Some(entry.timestamp);

                // Get preview from first user message
                if preview_text.is_empty() && entry.entry_type == "user" {
                    if let Some(msg) = &entry.message {
                        let text = msg.text();
                        // Sanitize: replace newlines and control chars with spaces
                        let sanitized: String = text
                            .chars()
                            .map(|c| if c.is_control() { ' ' } else { c })
                            .collect();
                        let sanitized = sanitized.trim();
                        // Truncate to reasonable preview length
                        preview_text = if sanitized.len() > 200 {
                            format!("{}...", &sanitized[..200])
                        } else {
                            sanitized.to_string()
                        };
                    }
                }
            }
        }

        // Handle empty or invalid sessions
        let first_message =
            first_message.context("Session has no messages")?;
        let last_message = last_message.unwrap_or(first_message);

        // Decode project path from directory name if not found in entries
        if project_path.is_empty() {
            if let Some(parent) = file_path.parent() {
                if let Some(dir_name) = parent.file_name().and_then(|n| n.to_str()) {
                    project_path = decode_project_path(dir_name);
                }
            }
        }

        Ok(Session {
            id: session_id,
            project_path,
            slug,
            git_branch,
            first_message,
            last_message,
            message_count,
            preview_text,
            file_path,
            file_size: metadata.len(),
            is_agent,
            agent_id,
        })
    }

    /// Get all discovered projects.
    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    /// Get sessions for a specific project.
    pub fn sessions_for_project(&self, encoded_path: &str) -> Option<&Vec<Session>> {
        self.sessions.get(encoded_path)
    }

    /// Get total session count across all projects.
    pub fn total_session_count(&self) -> usize {
        self.sessions.values().map(|s| s.len()).sum()
    }
}
