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
///
/// The encoding is lossy (hyphens become indistinguishable from path separators),
/// so we try different decodings and return the one that exists on disk.
pub fn decode_project_path(encoded: &str) -> String {
    if encoded.is_empty() {
        return String::new();
    }

    // Find all hyphen positions (except the leading one which is always a /)
    let chars: Vec<char> = encoded.chars().collect();
    let mut hyphen_positions: Vec<usize> = Vec::new();

    for (i, &c) in chars.iter().enumerate() {
        if c == '-' && i > 0 {
            hyphen_positions.push(i);
        }
    }

    // Try all combinations of converting hyphens to slashes vs keeping them
    // Start with the most slashes (all hyphens become slashes) and work down
    // Return the first path that exists
    let num_hyphens = hyphen_positions.len();

    // Try from all-slashes down to no-slashes-except-leading
    for num_to_convert in (0..=num_hyphens).rev() {
        // Generate all combinations of `num_to_convert` positions to convert
        for positions_to_convert in combinations(&hyphen_positions, num_to_convert) {
            let convert_set: std::collections::HashSet<usize> = positions_to_convert.into_iter().collect();

            let mut result = String::new();
            for (i, &c) in chars.iter().enumerate() {
                if c == '-' {
                    if i == 0 || convert_set.contains(&i) {
                        result.push('/');
                    } else {
                        result.push('-');
                    }
                } else {
                    result.push(c);
                }
            }

            // Check if this path exists
            if std::path::Path::new(&result).exists() {
                return result;
            }
        }
    }

    // Fallback: just convert all hyphens to slashes
    encoded.replace('-', "/")
}

/// Generate all combinations of `k` elements from `items`.
fn combinations<T: Clone>(items: &[T], k: usize) -> Vec<Vec<T>> {
    if k == 0 {
        return vec![vec![]];
    }
    if items.len() < k {
        return vec![];
    }
    if items.len() == k {
        return vec![items.to_vec()];
    }

    let mut result = Vec::new();

    // Include first element
    for mut combo in combinations(&items[1..], k - 1) {
        combo.insert(0, items[0].clone());
        result.push(combo);
    }

    // Exclude first element
    result.extend(combinations(&items[1..], k));

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
    fn test_decode_project_path_existing() {
        // Test with paths that exist on the system
        // /etc/nixos should exist and decode correctly
        let decoded = decode_project_path("-etc-nixos");
        assert!(decoded == "/etc/nixos" || decoded == "/etc/nixos");
    }

    #[test]
    fn test_decode_project_path_fallback() {
        // For non-existent paths, falls back to simple replacement
        assert_eq!(
            decode_project_path("-nonexistent-path-here"),
            "/nonexistent/path/here"
        );
    }

    #[test]
    fn test_encode_project_path() {
        assert_eq!(
            encode_project_path("/home/garrett/Projects/jwst-cosmos"),
            "-home-garrett-Projects-jwst-cosmos"
        );
    }

    #[test]
    fn test_combinations() {
        let items = vec![1, 2, 3];
        let empty: Vec<Vec<i32>> = vec![vec![]];
        assert_eq!(combinations(&items, 0), empty);
        assert_eq!(combinations(&items, 1), vec![vec![1], vec![2], vec![3]]);
        assert_eq!(combinations(&items, 2), vec![vec![1, 2], vec![1, 3], vec![2, 3]]);
        assert_eq!(combinations(&items, 3), vec![vec![1, 2, 3]]);
    }
}
