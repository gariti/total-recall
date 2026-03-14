//! jj workspace lifecycle management for agent worktrees.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Manages jj workspaces for isolated agent work.
pub struct WorktreeManager;

impl WorktreeManager {
    /// Create a jj workspace for an agent.
    ///
    /// Runs `jj workspace add <path> --name <name>` from the project root.
    /// Returns the worktree path.
    pub fn create(project_path: &Path, agent_name: &str) -> Result<PathBuf> {
        let project_name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project");

        let worktree_path = PathBuf::from(format!("/tmp/tr-{}-{}", project_name, agent_name));
        let workspace_name = format!("tr-{}", agent_name);

        // Create the workspace
        let output = Command::new("jj")
            .arg("workspace")
            .arg("add")
            .arg(worktree_path.to_str().unwrap())
            .arg("--name")
            .arg(&workspace_name)
            .current_dir(project_path)
            .output()
            .context("Failed to run jj workspace add")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("jj workspace add failed: {}", stderr);
        }

        tracing::info!("Created worktree at {:?} for agent {}", worktree_path, agent_name);
        Ok(worktree_path)
    }

    /// Destroy a jj workspace and clean up the directory.
    ///
    /// Runs `jj workspace forget <name>` from the project root, then removes the dir.
    pub fn destroy(project_path: &Path, agent_name: &str, worktree_path: &Path) -> Result<()> {
        let workspace_name = format!("tr-{}", agent_name);

        // Forget the workspace
        let output = Command::new("jj")
            .arg("workspace")
            .arg("forget")
            .arg(&workspace_name)
            .current_dir(project_path)
            .output()
            .context("Failed to run jj workspace forget")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!("jj workspace forget failed (may already be gone): {}", stderr);
        }

        // Remove the directory
        if worktree_path.exists() {
            std::fs::remove_dir_all(worktree_path)
                .with_context(|| format!("Failed to remove worktree dir {:?}", worktree_path))?;
            tracing::info!("Removed worktree directory {:?}", worktree_path);
        }

        Ok(())
    }
}
