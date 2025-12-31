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
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::path::Path;

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

/// Detect the terminal emulator to use.
fn detect_terminal() -> String {
    // Check TERMINAL env var first
    if let Ok(term) = std::env::var("TERMINAL") {
        // Extract just the binary name
        let name = Path::new(&term)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&term);
        return name.to_string();
    }

    // Try to detect from common terminals (wezterm preferred)
    let terminals = ["wezterm", "kitty", "alacritty", "foot", "gnome-terminal", "konsole", "xterm"];
    for term in terminals {
        if std::process::Command::new("which")
            .arg(term)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return term.to_string();
        }
    }

    "xterm".to_string()
}

/// Escape a string for shell usage.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
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

    // Run the TUI application
    let mut app = app::App::new(config).await?;
    match app.run().await? {
        app::AppResult::Exit => {}
        app::AppResult::NewSession { project_path } => {
            // Spawn a new terminal with a fresh claude session using tmux for persistent header
            let terminal = detect_terminal();
            let tmux_session = build_tmux_session_name(&project_path, None);
            let status_text = build_tmux_status(&project_path, None);

            // Build tmux command with status bar at top (hide window list)
            // Run claude directly as the session command instead of send-keys
            // Use -A to attach to existing session or create new (avoids "duplicate session" error)
            let tmux_cmd = format!(
                "tmux new-session -A -s {} -c {} 'claude; echo; echo Press Enter to close...; read' \\; set status on \\; set status-position top \\; set status-style 'bg=blue,fg=white,bold' \\; set status-left-length 100 \\; set status-left ' {} ' \\; set status-right '' \\; set window-status-format '' \\; set window-status-current-format ''",
                shell_escape(&tmux_session),
                shell_escape(&project_path),
                status_text.replace('\'', "'\\''")
            );

            let result = match terminal.as_str() {
                "kitty" => std::process::Command::new("kitty")
                    .arg("--detach")
                    .arg("--directory")
                    .arg(&project_path)
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "alacritty" => std::process::Command::new("alacritty")
                    .arg("-e")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "foot" => std::process::Command::new("foot")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "wezterm" => {
                    let log_path = format!("{}/.total-recall-spawn.log", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));

                    // Write tmux command to script to avoid quoting issues
                    let script_path = format!("{}/.total-recall-launch.sh", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
                    let script_content = format!(
                        "#!/bin/bash\ncd '{}'\n{}\n",
                        project_path.replace('\'', "'\\''"),
                        tmux_cmd
                    );
                    let _ = std::fs::write(&script_path, &script_content);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));
                    }

                    let spawn_cmd = format!(
                        "nohup wezterm start --always-new-process --cwd '{}' -- bash '{}' >/dev/null 2>&1 &",
                        project_path.replace('\'', "'\\''"),
                        script_path
                    );

                    // Log for debugging
                    let _ = std::fs::write(&log_path, format!(
                        "=== NEW SESSION ===\nproject_path: {}\ntmux_session: {}\nscript:\n{}\nspawn_cmd: {}\n",
                        project_path, tmux_session, script_content, spawn_cmd
                    ));

                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&spawn_cmd)
                        .spawn();
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    result
                }
                "gnome-terminal" => std::process::Command::new("gnome-terminal")
                    .arg("--")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "konsole" => std::process::Command::new("konsole")
                    .arg("-e")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "xterm" => std::process::Command::new("xterm")
                    .arg("-e")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                _ => std::process::Command::new("x-terminal-emulator")
                    .arg("-e")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
            };

            if let Err(e) = result {
                eprintln!("Failed to launch terminal '{}': {}", terminal, e);
                eprintln!("Falling back to exec in current terminal...");

                if let Err(e) = std::env::set_current_dir(&project_path) {
                    eprintln!("Failed to change to project directory '{}': {}", project_path, e);
                    std::process::exit(1);
                }
                use std::os::unix::process::CommandExt;
                let err = std::process::Command::new("claude")
                    .exec();
                eprintln!("Failed to launch claude: {}", err);
                std::process::exit(1);
            }
        }
        app::AppResult::LaunchSession { session_id, project_path } => {
            // Spawn a new terminal with the claude resume command using tmux for persistent header
            let terminal = detect_terminal();
            let tmux_session = build_tmux_session_name(&project_path, Some(&session_id));
            let status_text = build_tmux_status(&project_path, Some(&session_id));

            // Log to file for debugging
            let log_path = format!("{}/.total-recall-spawn.log", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
            let _ = std::fs::write(&log_path, format!(
                "=== LAUNCH ===\nterminal: {}\nsession_id: {}\nproject_path: {}\ntmux_session: {}\n",
                terminal, session_id, project_path, tmux_session
            ));

            // Build tmux command with status bar at top (hide window list)
            // Run claude directly as the session command instead of send-keys
            // Note: session_id is inside single quotes in the command, so escape quotes instead of wrapping
            // Use -A to attach to existing session or create new (avoids "duplicate session" error)
            let tmux_cmd = format!(
                "tmux new-session -A -s {} -c {} 'claude --resume {}; echo; echo Press Enter to close...; read' \\; set status on \\; set status-position top \\; set status-style 'bg=blue,fg=white,bold' \\; set status-left-length 100 \\; set status-left ' {} ' \\; set status-right '' \\; set window-status-format '' \\; set window-status-current-format ''",
                shell_escape(&tmux_session),
                shell_escape(&project_path),
                session_id.replace('\'', "'\\''"),
                status_text.replace('\'', "'\\''")
            );

            let result = match terminal.as_str() {
                "kitty" => std::process::Command::new("kitty")
                    .arg("--detach")
                    .arg("--directory")
                    .arg(&project_path)
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "alacritty" => std::process::Command::new("alacritty")
                    .arg("-e")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "foot" => std::process::Command::new("foot")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "wezterm" => {
                    use std::io::Write;
                    let log_path = format!("{}/.total-recall-spawn.log", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));

                    // Write tmux command to script to avoid quoting issues
                    let script_path = format!("{}/.total-recall-launch.sh", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
                    let script_content = format!(
                        "#!/bin/bash\ncd '{}'\n{}\n",
                        project_path.replace('\'', "'\\''"),
                        tmux_cmd
                    );
                    let _ = std::fs::write(&script_path, &script_content);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755));
                    }

                    let spawn_cmd = format!(
                        "nohup wezterm start --always-new-process --cwd '{}' -- bash '{}' >/dev/null 2>&1 &",
                        project_path.replace('\'', "'\\''"),
                        script_path
                    );

                    // Append to log
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
                        let _ = writeln!(f, "tmux_cmd: {}", tmux_cmd);
                        let _ = writeln!(f, "script_content:\n{}", script_content);
                        let _ = writeln!(f, "spawn_cmd: {}", spawn_cmd);
                    }

                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&spawn_cmd)
                        .spawn();

                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
                        let _ = writeln!(f, "spawn result: {:?}", result.as_ref().map(|c| c.id()));
                    }

                    std::thread::sleep(std::time::Duration::from_millis(200));
                    result
                }
                "gnome-terminal" => std::process::Command::new("gnome-terminal")
                    .arg("--")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "konsole" => std::process::Command::new("konsole")
                    .arg("-e")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                "xterm" => std::process::Command::new("xterm")
                    .arg("-e")
                    .arg("bash")
                    .arg("-c")
                    .arg(&tmux_cmd)
                    .spawn(),
                _ => {
                    // Fallback: try x-terminal-emulator
                    std::process::Command::new("x-terminal-emulator")
                        .arg("-e")
                        .arg("bash")
                        .arg("-c")
                        .arg(&tmux_cmd)
                        .spawn()
                }
            };

            if let Err(e) = result {
                eprintln!("Failed to launch terminal '{}': {}", terminal, e);
                eprintln!("Falling back to exec in current terminal...");

                // Fallback to original behavior
                if let Err(e) = std::env::set_current_dir(&project_path) {
                    eprintln!("Failed to change to project directory '{}': {}", project_path, e);
                    std::process::exit(1);
                }
                use std::os::unix::process::CommandExt;
                let err = std::process::Command::new("claude")
                    .arg("--resume")
                    .arg(&session_id)
                    .exec();
                eprintln!("Failed to launch claude: {}", err);
                std::process::exit(1);
            }
            // New terminal spawned successfully, exit this instance
        }
        app::AppResult::OpenLazygit { project_path } => {
            // Spawn a terminal with lazygit in the project directory
            let terminal = detect_terminal();

            let result = match terminal.as_str() {
                "kitty" => {
                    std::process::Command::new("kitty")
                        .arg("--detach")
                        .arg("--directory")
                        .arg(&project_path)
                        .arg("lazygit")
                        .spawn()
                }
                "wezterm" => {
                    let spawn_cmd = format!(
                        "nohup wezterm start --always-new-process --cwd '{}' -- lazygit >/dev/null 2>&1 &",
                        project_path.replace('\'', "'\\''")
                    );
                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&spawn_cmd)
                        .spawn();
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    result
                }
                "alacritty" => std::process::Command::new("alacritty")
                    .arg("--working-directory")
                    .arg(&project_path)
                    .arg("-e")
                    .arg("lazygit")
                    .spawn(),
                "foot" => std::process::Command::new("foot")
                    .arg("--working-directory")
                    .arg(&project_path)
                    .arg("lazygit")
                    .spawn(),
                _ => {
                    let cmd = format!("cd {} && lazygit", shell_escape(&project_path));
                    std::process::Command::new("x-terminal-emulator")
                        .arg("-e")
                        .arg("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .spawn()
                }
            };

            if let Err(e) = result {
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
            // Spawn a terminal in the project directory
            let terminal = detect_terminal();

            let result = match terminal.as_str() {
                "kitty" => {
                    std::process::Command::new("kitty")
                        .arg("--detach")
                        .arg("--directory")
                        .arg(&project_path)
                        .spawn()
                }
                "wezterm" => {
                    let spawn_cmd = format!(
                        "nohup wezterm start --always-new-process --cwd '{}' >/dev/null 2>&1 &",
                        project_path.replace('\'', "'\\''")
                    );
                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&spawn_cmd)
                        .spawn();
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    result
                }
                "alacritty" => std::process::Command::new("alacritty")
                    .arg("--working-directory")
                    .arg(&project_path)
                    .spawn(),
                "foot" => std::process::Command::new("foot")
                    .arg("--working-directory")
                    .arg(&project_path)
                    .spawn(),
                "gnome-terminal" => std::process::Command::new("gnome-terminal")
                    .arg("--working-directory")
                    .arg(&project_path)
                    .spawn(),
                "konsole" => std::process::Command::new("konsole")
                    .arg("--workdir")
                    .arg(&project_path)
                    .spawn(),
                _ => {
                    let cmd = format!("cd {}", shell_escape(&project_path));
                    std::process::Command::new("x-terminal-emulator")
                        .arg("-e")
                        .arg("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .spawn()
                }
            };

            if let Err(e) = result {
                eprintln!("Failed to open terminal: {}", e);
            }
        }
        app::AppResult::OpenEditor { project_path } => {
            // Get editor from $EDITOR env var
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
            let terminal = detect_terminal();

            let result = match terminal.as_str() {
                "kitty" => {
                    std::process::Command::new("kitty")
                        .arg("--detach")
                        .arg("--directory")
                        .arg(&project_path)
                        .arg(&editor)
                        .spawn()
                }
                "wezterm" => {
                    let spawn_cmd = format!(
                        "nohup wezterm start --always-new-process --cwd '{}' -- {} >/dev/null 2>&1 &",
                        project_path.replace('\'', "'\\''"),
                        editor
                    );
                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&spawn_cmd)
                        .spawn();
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    result
                }
                "alacritty" => std::process::Command::new("alacritty")
                    .arg("--working-directory")
                    .arg(&project_path)
                    .arg("-e")
                    .arg(&editor)
                    .spawn(),
                "foot" => std::process::Command::new("foot")
                    .arg("--working-directory")
                    .arg(&project_path)
                    .arg(&editor)
                    .spawn(),
                _ => {
                    let cmd = format!("cd {} && {}", shell_escape(&project_path), editor);
                    std::process::Command::new("x-terminal-emulator")
                        .arg("-e")
                        .arg("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .spawn()
                }
            };

            if let Err(e) = result {
                eprintln!("Failed to open editor: {}", e);
            }
        }
    }

    Ok(())
}
