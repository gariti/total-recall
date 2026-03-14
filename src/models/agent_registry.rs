//! Deserializes the agent registry from /etc/nixos/agents/agent-registry.json.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Top-level registry structure.
#[derive(Debug, Deserialize)]
pub struct AgentRegistry {
    pub claude_code_agents: HashMap<String, AgentTypeEntry>,
}

/// A single agent type definition.
#[derive(Debug, Clone, Deserialize)]
pub struct AgentTypeEntry {
    pub name: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub use_cases: Vec<String>,
}

impl AgentRegistry {
    /// Load registry from the default path.
    pub fn load() -> anyhow::Result<Self> {
        Self::from_file("/etc/nixos/agents/agent-registry.json")
    }

    /// Load registry from a specific path.
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let registry: AgentRegistry = serde_json::from_str(&content)?;
        Ok(registry)
    }

    /// Get sorted list of agent type keys.
    pub fn type_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.claude_code_agents.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Look up a specific agent type.
    pub fn get(&self, key: &str) -> Option<&AgentTypeEntry> {
        self.claude_code_agents.get(key)
    }
}
