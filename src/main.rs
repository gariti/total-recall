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
            // Spawn a new terminal with a fresh claude session
            let terminal = detect_terminal();
            let claude_cmd = format!("cd {} && claude",
                shell_escape(&project_path));

            let result = match terminal.as_str() {
                "kitty" => {
                    let cmd = "claude; echo ''; echo 'Session ended. Press Enter to close...'; read";
                    std::process::Command::new("kitty")
                        .arg("--detach")
                        .arg("--directory")
                        .arg(&project_path)
                        .arg("bash")
                        .arg("-c")
                        .arg(cmd)
                        .spawn()
                }
                "alacritty" => std::process::Command::new("alacritty")
                    .arg("-e")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                "foot" => std::process::Command::new("foot")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                "wezterm" => {
                    let cmd = "claude; echo ''; echo 'Session ended. Press Enter to close...'; read";
                    let spawn_cmd = format!(
                        "nohup wezterm start --always-new-process --cwd '{}' -- bash -c '{}' >/dev/null 2>&1 &",
                        project_path.replace('\'', "'\\''"),
                        cmd.replace('\'', "'\\''")
                    );
                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&spawn_cmd)
                        .spawn();
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    result
                }
                "gnome-terminal" => std::process::Command::new("gnome-terminal")
                    .arg("--")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                "konsole" => std::process::Command::new("konsole")
                    .arg("-e")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                "xterm" => std::process::Command::new("xterm")
                    .arg("-e")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                _ => std::process::Command::new("x-terminal-emulator")
                    .arg("-e")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
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
            // Spawn a new terminal with the claude resume command
            let terminal = detect_terminal();

            // Log to file for debugging - use home dir to avoid any /tmp issues
            use std::io::Write;
            let log_path = format!("{}/.total-recall-spawn.log", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));
            let _ = std::fs::write(&log_path, format!(
                "=== LAUNCH ===\nterminal: {}\nsession_id: {}\nproject_path: {}\n",
                terminal, session_id, project_path
            ));
            let claude_cmd = format!("cd {} && claude --resume {}",
                shell_escape(&project_path),
                shell_escape(&session_id));

            let result = match terminal.as_str() {
                "kitty" => {
                    let cmd = format!(
                        "cd {} && claude --resume {}; echo ''; echo 'Session ended. Press Enter to close...'; read",
                        shell_escape(&project_path),
                        shell_escape(&session_id)
                    );
                    std::process::Command::new("kitty")
                        .arg("--detach")
                        .arg("--directory")
                        .arg(&project_path)
                        .arg("bash")
                        .arg("-c")
                        .arg(&cmd)
                        .spawn()
                }
                "alacritty" => std::process::Command::new("alacritty")
                    .arg("-e")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                "foot" => std::process::Command::new("foot")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                "wezterm" => {
                    use std::io::Write;
                    let log_path = format!("{}/.total-recall-spawn.log", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()));

                    let cmd = format!(
                        "claude --resume {}; echo ''; echo 'Session ended. Press Enter to close...'; read",
                        shell_escape(&session_id)
                    );

                    // Use sh -c with nohup and & to fully detach from parent
                    let spawn_cmd = format!(
                        "nohup wezterm start --always-new-process --cwd '{}' -- bash -c '{}' >/dev/null 2>&1 &",
                        project_path.replace('\'', "'\\''"),
                        cmd.replace('\'', "'\\''")
                    );

                    // Append to log
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
                        let _ = writeln!(f, "spawn_cmd: {}", spawn_cmd);
                    }

                    let result = std::process::Command::new("sh")
                        .arg("-c")
                        .arg(&spawn_cmd)
                        .spawn();

                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
                        let _ = writeln!(f, "spawn result: {:?}", result.as_ref().map(|c| c.id()));
                    }

                    // Small delay to ensure child starts before we exit
                    std::thread::sleep(std::time::Duration::from_millis(200));

                    result
                }
                "gnome-terminal" => std::process::Command::new("gnome-terminal")
                    .arg("--")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                "konsole" => std::process::Command::new("konsole")
                    .arg("-e")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                "xterm" => std::process::Command::new("xterm")
                    .arg("-e")
                    .arg("sh")
                    .arg("-c")
                    .arg(&claude_cmd)
                    .spawn(),
                _ => {
                    // Fallback: try x-terminal-emulator or just run in same terminal
                    std::process::Command::new("x-terminal-emulator")
                        .arg("-e")
                        .arg("sh")
                        .arg("-c")
                        .arg(&claude_cmd)
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
    }

    Ok(())
}
