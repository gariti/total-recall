//! Project grouping for sessions.

use chrono::{DateTime, Utc};

/// A project that contains Claude Code sessions.
#[derive(Debug, Clone)]
pub struct Project {
    /// Encoded path (directory name, e.g., "-home-garrett-Projects-jwst-cosmos")
    pub encoded_path: String,
    /// Decoded path (e.g., "/home/garrett/Projects/jwst-cosmos")
    pub decoded_path: String,
    /// Display name (last path component, e.g., "jwst-cosmos")
    pub display_name: String,
    /// Number of sessions in this project
    pub session_count: usize,
    /// Total messages across all sessions
    pub total_messages: usize,
    /// Most recent activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl Project {
    /// Create a new project from an encoded path.
    pub fn new(encoded_path: String) -> Self {
        let decoded_path = decode_project_path(&encoded_path);
        let display_name = decoded_path
            .rsplit('/')
            .next()
            .unwrap_or(&decoded_path)
            .to_string();

        Self {
            encoded_path,
            decoded_path,
            display_name,
            session_count: 0,
            total_messages: 0,
            last_activity: DateTime::<Utc>::MIN_UTC,
        }
    }
}

/// Decode a Claude project path from directory name format.
/// e.g., "-home-garrett-Projects-jwst-cosmos" -> "/home/garrett/Projects/jwst-cosmos"
pub fn decode_project_path(encoded: &str) -> String {
    if encoded.is_empty() {
        return String::new();
    }

    // Replace leading dash with /
    // Then replace remaining dashes with /
    // But be careful about multiple consecutive dashes
    let mut result = String::new();
    let mut chars = encoded.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '-' {
            result.push('/');
        } else {
            result.push(c);
        }
    }

    result
}

/// Encode a project path to Claude directory name format.
/// e.g., "/home/garrett/Projects/jwst-cosmos" -> "-home-garrett-Projects-jwst-cosmos"
pub fn encode_project_path(path: &str) -> String {
    path.replace('/', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_project_path() {
        assert_eq!(
            decode_project_path("-home-garrett-Projects-jwst-cosmos"),
            "/home/garrett/Projects/jwst-cosmos"
        );
        assert_eq!(decode_project_path("-etc-nixos"), "/etc/nixos");
    }

    #[test]
    fn test_encode_project_path() {
        assert_eq!(
            encode_project_path("/home/garrett/Projects/jwst-cosmos"),
            "-home-garrett-Projects-jwst-cosmos"
        );
    }
}
