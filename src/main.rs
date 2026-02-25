//! total-recall - Claude Sessions TUI
//!
//! "Get your ass to Claude." - Quaid, probably
//!
//! A terminal-based application for browsing and resuming Claude Code
//! conversations across all your projects.

mod app;
mod config;
mod models;
mod screens;
mod services;
mod utils;

use anyhow::Result;
use clap::Parser;
use std::os::unix::process::CommandExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Spawn a command detached from the current terminal session.
///
/// Uses setsid() in a pre_exec hook to create a new session, so the child
/// won't receive SIGHUP when the parent terminal (e.g. quake ghostty) closes.
fn spawn_detached(cmd: &mut std::process::Command) -> std::io::Result<std::process::Child> {
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
}

/// total-recall - Claude Sessions Browser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Config file path (default: ~/.config/total-recall/config.toml)
    #[arg(short, long)]
    config: Option<String>,

    /// Claude directory path (default: ~/.claude)
    #[arg(long)]
    claude_dir: Option<String>,
}

/// Build a ghostty command for spawning a new terminal window.
///
/// Uses plain `ghostty` (new process) rather than `+new-window` (D-Bus IPC).
/// In ghostty 1.3.0-dev, `+new-window -e` silently ignores the command —
/// the window opens with the default shell instead. Plain `ghostty` supports
/// all flags: -e, --working-directory, --class, --font-size, etc.
fn ghostty_command() -> std::process::Command {
    std::process::Command::new("ghostty")
}

/// Append tmux status bar configuration args to a command.
fn append_tmux_status_args(cmd: &mut std::process::Command, status_text: &str) {
    cmd.arg(";").arg("set").arg("status").arg("on")
        .arg(";").arg("set").arg("status-position").arg("top")
        .arg(";").arg("set").arg("status-style").arg("bg=blue,fg=white,bold")
        .arg(";").arg("set").arg("status-left-length").arg("100")
        .arg(";").arg("set").arg("status-left").arg(format!(" {} ", status_text))
        .arg(";").arg("set").arg("status-right").arg("")
        .arg(";").arg("set").arg("window-status-format").arg("")
        .arg(";").arg("set").arg("window-status-current-format").arg("");
}

/// Build a tmux session name from project path and session ID.
fn build_tmux_session_name(project_path: &str, session_id: Option<&str>) -> String {
    let name = std::path::Path::new(project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("session");

    match session_id {
        Some(id) => format!("tr-{}-{}", name, &id[..8.min(id.len())]),
        None => {
            // Use timestamp to ensure unique session names for new sessions
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            format!("tr-{}-{}", name, ts)
        }
    }
}

/// Build the tmux status line content showing project and session info.
fn build_tmux_status(project_path: &str, session_id: Option<&str>) -> String {
    let name = std::path::Path::new(project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(project_path);

    match session_id {
        Some(id) => format!("[{}] {}", name, &id[..8.min(id.len())]),
        None => format!("[{}]", name),
    }
}

/// Parse a git remote URL to a GitHub browser URL.
fn git_remote_to_github_url(remote: &str) -> Option<String> {
    let remote = remote.trim();

    // Handle SSH format: git@github.com:user/repo.git
    if let Some(rest) = remote.strip_prefix("git@github.com:") {
        let repo = rest.trim_end_matches(".git");
        return Some(format!("https://github.com/{}", repo));
    }

    // Handle HTTPS format: https://github.com/user/repo.git
    if remote.starts_with("https://github.com/") {
        let url = remote.trim_end_matches(".git");
        return Some(url.to_string());
    }

    // Handle other git hosts similarly if needed
    None
}

/// Get the git remote origin URL for a project.
fn get_git_remote_url(project_path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(project_path)
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set up logging
    let filter = if args.debug {
        "total_recall=debug,info"
    } else {
        "total_recall=info,warn"
    };

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();

    // Load configuration
    let mut config = if let Some(path) = args.config {
        config::Config::from_file(&path)?
    } else {
        config::Config::load()?
    };

    // Override claude_dir if specified
    if let Some(claude_dir) = args.claude_dir {
        config.claude.claude_dir = claude_dir;
    }

    let skip_perms = config.claude.dangerously_skip_permissions;
    let skip_perms_flag = if skip_perms { " --dangerously-skip-permissions" } else { "" };

    // Run the TUI application
    let mut app = app::App::new(config).await?;
    match app.run().await? {
        app::AppResult::Exit => {}
        app::AppResult::NewSession { project_path } => {
            let tmux_session = build_tmux_session_name(&project_path, None);
            let status_text = build_tmux_status(&project_path, None);

            let mut cmd = ghostty_command();
            cmd.arg("-e")
                .arg("tmux").arg("new-session").arg("-A")
                .arg("-s").arg(&tmux_session)
                .arg("-c").arg(&project_path)
                .arg(format!("claude{skip_perms_flag}; echo; echo Press Enter to close...; read"));
            append_tmux_status_args(&mut cmd, &status_text);

            if let Err(e) = spawn_detached(&mut cmd) {
                eprintln!("Failed to launch terminal: {}", e);
                eprintln!("Falling back to running in current terminal...");

                if let Err(e) = std::env::set_current_dir(&project_path) {
                    eprintln!("Failed to change to project directory '{}': {}", project_path, e);
                    std::process::exit(1);
                }
                let mut fallback_cmd = std::process::Command::new("claude");
                if skip_perms { fallback_cmd.arg("--dangerously-skip-permissions"); }
                let err = fallback_cmd.exec();
                eprintln!("Failed to launch claude: {}", err);
                std::process::exit(1);
            }
        }
        app::AppResult::LaunchSession { session_id, project_path } => {
            let tmux_session = build_tmux_session_name(&project_path, Some(&session_id));
            let status_text = build_tmux_status(&project_path, Some(&session_id));

            let resume_cmd = format!("claude{skip_perms_flag} --resume {session_id}; echo; echo Press Enter to close...; read");
            let mut cmd = ghostty_command();
            cmd.arg("-e")
                .arg("tmux").arg("new-session").arg("-A")
                .arg("-s").arg(&tmux_session)
                .arg("-c").arg(&project_path)
                .arg(&resume_cmd);
            append_tmux_status_args(&mut cmd, &status_text);

            if let Err(e) = spawn_detached(&mut cmd) {
                eprintln!("Failed to launch terminal: {}", e);
                eprintln!("Falling back to running in current terminal...");

                if let Err(e) = std::env::set_current_dir(&project_path) {
                    eprintln!("Failed to change to project directory '{}': {}", project_path, e);
                    std::process::exit(1);
                }
                let mut fallback_cmd = std::process::Command::new("claude");
                if skip_perms { fallback_cmd.arg("--dangerously-skip-permissions"); }
                let err = fallback_cmd.arg("--resume").arg(&session_id).exec();
                eprintln!("Failed to launch claude: {}", err);
                std::process::exit(1);
            }
        }
        app::AppResult::OpenLazygit { project_path } => {
            let tmux_session = build_tmux_session_name(&project_path, None);
            let mut cmd = ghostty_command();
            cmd.arg("-e")
                .arg("tmux").arg("new-session").arg("-A")
                .arg("-s").arg(&tmux_session)
                .arg("-c").arg(&project_path)
                .arg("lazygit");

            if let Err(e) = spawn_detached(&mut cmd) {
                eprintln!("Failed to launch lazygit: {}", e);
            }
        }
        app::AppResult::OpenGithub { project_path } => {
            // Get git remote and open in browser
            if let Some(remote) = get_git_remote_url(&project_path) {
                if let Some(url) = git_remote_to_github_url(&remote) {
                    if let Err(e) = open::that_detached(&url) {
                        eprintln!("Failed to open browser: {}", e);
                    }
                } else {
                    eprintln!("Could not parse git remote URL: {}", remote.trim());
                }
            } else {
                eprintln!("No git remote origin found for: {}", project_path);
            }
        }
        app::AppResult::OpenTerminal { project_path } => {
            let tmux_session = build_tmux_session_name(&project_path, None);
            let mut cmd = ghostty_command();
            cmd.arg("-e")
                .arg("tmux").arg("new-session").arg("-A")
                .arg("-s").arg(&tmux_session)
                .arg("-c").arg(&project_path);

            if let Err(e) = spawn_detached(&mut cmd) {
                eprintln!("Failed to open terminal: {}", e);
            }
        }
        app::AppResult::OpenEditor { project_path } => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
            let tmux_session = build_tmux_session_name(&project_path, None);
            let mut cmd = ghostty_command();
            cmd.arg("-e")
                .arg("tmux").arg("new-session").arg("-A")
                .arg("-s").arg(&tmux_session)
                .arg("-c").arg(&project_path)
                .arg(&editor);

            if let Err(e) = spawn_detached(&mut cmd) {
                eprintln!("Failed to open editor: {}", e);
            }
        }
    }

    Ok(())
}
