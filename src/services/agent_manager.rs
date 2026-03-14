//! Core agent orchestration — spawn, monitor, kill, persist.

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::event::AppEvent;
use crate::models::agent::{Agent, AgentStatus};
use crate::models::agent_registry::AgentRegistry;
use crate::services::worktree_manager::WorktreeManager;

/// Persistence file for agents across restarts.
fn agents_file() -> PathBuf {
    Config::data_dir().join("agents.json")
}

/// Manages the lifecycle of all spawned agents.
pub struct AgentManager {
    agents: Vec<Agent>,
    registry: AgentRegistry,
    config: std::sync::Arc<Config>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
}

impl AgentManager {
    /// Create a new manager, loading persisted agents and the registry.
    pub fn new(
        config: std::sync::Arc<Config>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
    ) -> Result<Self> {
        let registry = AgentRegistry::load().unwrap_or_else(|e| {
            tracing::warn!("Failed to load agent registry: {}", e);
            AgentRegistry {
                claude_code_agents: HashMap::new(),
            }
        });

        let agents = Self::load_persisted().unwrap_or_default();

        Ok(Self {
            agents,
            registry,
            config,
            event_tx,
        })
    }

    /// Access the registry.
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// All tracked agents (alive + dead).
    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }

    /// Mutable access to agents.
    pub fn agents_mut(&mut self) -> &mut Vec<Agent> {
        &mut self.agents
    }

    /// Count of agents that are still alive.
    pub fn active_count(&self) -> usize {
        self.agents.iter().filter(|a| a.status.is_alive()).count()
    }

    /// Get an agent by index.
    pub fn get(&self, index: usize) -> Option<&Agent> {
        self.agents.get(index)
    }

    /// Get mutable agent by ID.
    pub fn get_by_id_mut(&mut self, id: &str) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|a| a.id == id)
    }

    /// Spawn a new agent.
    ///
    /// 1. Optionally create a jj worktree
    /// 2. Create a tmux session
    /// 3. Send the claude command
    pub fn spawn(
        &mut self,
        project_path: PathBuf,
        agent_type: String,
        task_prompt: String,
        use_worktree: bool,
    ) -> Result<usize> {
        let id = uuid::Uuid::new_v4().to_string();
        let short_id = &id[..8];

        // Generate a name from agent type + short id
        let name = format!("{}-{}", agent_type, short_id);
        let tmux_session = format!("tr-{}", name);

        // Determine working directory
        let (working_dir, worktree_path) = if use_worktree {
            let wt = WorktreeManager::create(&project_path, &name)?;
            let dir = wt.clone();
            (dir, Some(wt))
        } else {
            (project_path.clone(), None)
        };

        // Create tmux session
        let output = Command::new("tmux")
            .arg("new-session")
            .arg("-d")
            .arg("-s")
            .arg(&tmux_session)
            .arg("-c")
            .arg(working_dir.to_str().unwrap_or("."))
            .output()
            .context("Failed to create tmux session")?;

        if !output.status.success() {
            // Clean up worktree if tmux failed
            if let Some(ref wt) = worktree_path {
                let _ = WorktreeManager::destroy(&project_path, &name, wt);
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("tmux new-session failed: {}", stderr);
        }

        // Source wallust colors
        let _ = Command::new("tmux")
            .arg("send-keys")
            .arg("-t")
            .arg(&tmux_session)
            .arg("source ~/.cache/wallust/tmux.conf 2>/dev/null; clear")
            .arg("Enter")
            .output();

        // Small delay to let the source command finish
        std::thread::sleep(Duration::from_millis(200));

        // Build and send the claude command
        // Always skip permissions for spawned agents — they run unattended in tmux
        let claude_cmd = format!("claude --dangerously-skip-permissions '{}'", task_prompt.replace('\'', "'\\''"));

        let _ = Command::new("tmux")
            .arg("send-keys")
            .arg("-t")
            .arg(&tmux_session)
            .arg(&claude_cmd)
            .arg("Enter")
            .output();

        let now = Utc::now();
        let agent = Agent {
            id,
            name,
            agent_type,
            project_path,
            worktree_path,
            tmux_session,
            claude_session_id: None,
            status: AgentStatus::Starting,
            task_prompt,
            spawned_at: now,
            last_activity: now,
            message_count: 0,
            last_tool: None,
            last_output_lines: Vec::new(),
        };

        self.agents.push(agent);
        let index = self.agents.len() - 1;

        self.persist();
        tracing::info!("Spawned agent {} at index {}", self.agents[index].name, index);

        Ok(index)
    }

    /// Kill an agent's tmux session.
    ///
    /// Tolerant of already-dead agents: if the agent is already `Killed`,
    /// this is a no-op. If `Complete`/`Failed`, it transitions to `Killed`
    /// so the user sees the explosion animation.
    pub fn kill(&mut self, index: usize) -> Result<()> {
        let agent = self.agents.get_mut(index)
            .ok_or_else(|| anyhow::anyhow!("Agent index {} out of bounds", index))?;

        tracing::debug!("kill() called for agent {} (status: {:?})", agent.name, agent.status);

        if agent.status == AgentStatus::Killed {
            tracing::debug!("Agent {} already Killed, no-op", agent.name);
            return Ok(());
        }

        // Kill tmux session (harmless if already gone)
        let _ = Command::new("tmux")
            .arg("kill-session")
            .arg("-t")
            .arg(&agent.tmux_session)
            .output();

        agent.status = AgentStatus::Killed;
        agent.last_activity = Utc::now();
        let name = agent.name.clone();

        self.persist();
        tracing::info!("Killed agent {}", name);
        Ok(())
    }

    /// Delete an agent entirely (kill if alive, destroy worktree, remove from list).
    pub fn delete(&mut self, index: usize) -> Result<()> {
        if index >= self.agents.len() {
            anyhow::bail!("Agent index {} out of bounds", index);
        }

        let agent = &self.agents[index];

        // Kill if still alive
        if agent.status.is_alive() {
            let _ = Command::new("tmux")
                .arg("kill-session")
                .arg("-t")
                .arg(&agent.tmux_session)
                .output();
        }

        // Destroy worktree if it exists
        if let Some(ref wt) = agent.worktree_path {
            if let Err(e) = WorktreeManager::destroy(&agent.project_path, &agent.name, wt) {
                tracing::warn!("Failed to destroy worktree for {}: {}", agent.name, e);
            }
        }

        let name = self.agents[index].name.clone();
        self.agents.remove(index);
        self.persist();
        tracing::info!("Deleted agent {}", name);
        Ok(())
    }

    /// Poll all alive agents for status changes.
    ///
    /// Called periodically from the monitoring task.
    pub fn poll_agents(&mut self) {
        let mut events = Vec::new();

        for agent in &mut self.agents {
            if !agent.status.is_alive() {
                continue;
            }

            // Check if tmux session is still alive
            let alive = Command::new("tmux")
                .arg("has-session")
                .arg("-t")
                .arg(&agent.tmux_session)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !alive {
                let old_status = agent.status.clone();
                agent.status = if old_status == AgentStatus::Starting {
                    AgentStatus::Failed
                } else {
                    AgentStatus::Complete
                };
                agent.last_activity = Utc::now();
                events.push(AppEvent::AgentExited { agent_id: agent.id.clone() });
                continue;
            }

            // Capture last 10 lines from tmux pane
            if let Ok(output) = Command::new("tmux")
                .arg("capture-pane")
                .arg("-p")
                .arg("-t")
                .arg(&agent.tmux_session)
                .arg("-S")
                .arg("-10")
                .output()
            {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout);
                    let new_lines: Vec<String> = text
                        .lines()
                        .map(|l| l.to_string())
                        .filter(|l| !l.trim().is_empty())
                        .collect();

                    // Detect activity by comparing output
                    if new_lines != agent.last_output_lines {
                        agent.last_activity = Utc::now();
                        if agent.status != AgentStatus::Active {
                            agent.status = AgentStatus::Active;
                        }
                        agent.last_output_lines = new_lines;
                        events.push(AppEvent::AgentUpdate { agent_id: agent.id.clone() });
                    } else {
                        // No change — check if idle
                        let idle_secs = (Utc::now() - agent.last_activity).num_seconds();
                        if idle_secs > 30 && agent.status == AgentStatus::Active {
                            agent.status = AgentStatus::Idle;
                            events.push(AppEvent::AgentUpdate { agent_id: agent.id.clone() });
                        }
                    }
                }
            }

            // Try to discover JSONL session and extract tool info
            Self::try_discover_session(agent);
        }

        // Send all events
        for event in events {
            let _ = self.event_tx.send(event);
        }

        // Persist if anything changed
        self.persist();
    }

    /// Try to find the Claude session JSONL for an agent and extract metadata.
    fn try_discover_session(agent: &mut Agent) {
        if agent.claude_session_id.is_some() {
            // Already discovered — just try to read latest tool use
            Self::update_from_jsonl(agent);
            return;
        }

        // Look in ~/.claude/projects/ for JSONL files created after agent spawn
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return,
        };

        let working_dir = agent.worktree_path.as_ref().unwrap_or(&agent.project_path);
        let encoded = working_dir
            .to_str()
            .unwrap_or("")
            .replace('/', "-");

        let project_dir = home.join(".claude").join("projects").join(&encoded);
        if !project_dir.exists() {
            return;
        }

        // Find JSONL files created after spawn time
        let spawn_time = agent.spawned_at;
        if let Ok(entries) = std::fs::read_dir(&project_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                if let Ok(meta) = path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        let modified_chrono: chrono::DateTime<Utc> = modified.into();
                        if modified_chrono > spawn_time {
                            // Read first line to check cwd
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Some(first_line) = content.lines().next() {
                                    if let Ok(entry) = serde_json::from_str::<serde_json::Value>(first_line) {
                                        if let Some(session_id) = entry.get("sessionId").and_then(|v| v.as_str()) {
                                            agent.claude_session_id = Some(session_id.to_string());
                                            Self::update_from_jsonl(agent);
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Read the JSONL to get message count and last tool use.
    fn update_from_jsonl(agent: &mut Agent) {
        let session_id = match &agent.claude_session_id {
            Some(id) => id.clone(),
            None => return,
        };

        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return,
        };

        let working_dir = agent.worktree_path.as_ref().unwrap_or(&agent.project_path);
        let encoded = working_dir
            .to_str()
            .unwrap_or("")
            .replace('/', "-");

        let project_dir = home.join(".claude").join("projects").join(&encoded);
        let jsonl_path = project_dir.join(format!("{}.jsonl", session_id));

        if !jsonl_path.exists() {
            return;
        }

        if let Ok(content) = std::fs::read_to_string(&jsonl_path) {
            let mut msg_count = 0;
            let mut last_tool: Option<String> = None;

            for line in content.lines() {
                if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                    // Count assistant messages
                    if entry.get("type").and_then(|v| v.as_str()) == Some("assistant") {
                        msg_count += 1;
                    }

                    // Find tool_use in content blocks
                    if let Some(message) = entry.get("message") {
                        if let Some(content) = message.get("content") {
                            if let Some(blocks) = content.as_array() {
                                for block in blocks {
                                    if block.get("type").and_then(|v| v.as_str()) == Some("tool_use") {
                                        if let Some(name) = block.get("name").and_then(|v| v.as_str()) {
                                            last_tool = Some(name.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            agent.message_count = msg_count;
            if last_tool.is_some() {
                agent.last_tool = last_tool;
            }
        }
    }

    /// Start the background monitoring task.
    pub fn start_monitor(event_tx: mpsc::UnboundedSender<AppEvent>) -> mpsc::UnboundedSender<MonitorCommand> {
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<MonitorCommand>();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // The actual polling happens in App::handle_tick() which calls poll_agents()
                        // We just need to make sure ticks are flowing
                    }
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(MonitorCommand::Stop) | None => break,
                        }
                    }
                }
            }
        });

        cmd_tx
    }

    /// Persist agents to disk.
    fn persist(&self) {
        let dir = Config::data_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!("Failed to create data dir: {}", e);
            return;
        }
        let file = agents_file();
        match serde_json::to_string_pretty(&self.agents) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&file, json) {
                    tracing::warn!("Failed to persist agents: {}", e);
                }
            }
            Err(e) => tracing::warn!("Failed to serialize agents: {}", e),
        }
    }

    /// Load persisted agents from disk.
    fn load_persisted() -> Result<Vec<Agent>> {
        let file = agents_file();
        if !file.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&file)?;
        let agents: Vec<Agent> = serde_json::from_str(&content)?;
        Ok(agents)
    }
}

/// Commands for the background monitor task.
pub enum MonitorCommand {
    Stop,
}
