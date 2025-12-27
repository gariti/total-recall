# total-recall

> "Get your ass to Claude." - Quaid, probably

A terminal-based interface for browsing and resuming Claude Code conversations across all your projects.

## Features

- **Browse Sessions** - View all Claude Code sessions organized by project
- **Resume Conversations** - Launch sessions in a new terminal window with one keypress
- **Start New Sessions** - Begin fresh Claude conversations in any project
- **Project Actions** - Open lazygit, GitHub, terminal, or editor for any project
- **Clipboard Support** - Copy resume commands for manual pasting
- **Multi-Terminal Support** - Works with wezterm, kitty, alacritty, foot, gnome-terminal, konsole, and xterm

## Installation

### From Source

```bash
git clone https://github.com/yourusername/total-recall.git
cd total-recall
cargo build --release
cp target/release/total-recall ~/.local/bin/
```

### Requirements

- Rust 1.70+
- [Claude Code](https://claude.ai/claude-code) installed and configured

## Usage

```bash
# Launch the TUI
total-recall

# With debug logging
total-recall --debug

# Use a custom Claude directory
total-recall --claude-dir /path/to/.claude
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Navigate lists |
| `h` / `l` / `←` / `→` | Switch between project and session panes |
| `Enter` | Resume selected session in new terminal |
| `y` | Copy resume command to clipboard |
| `n` | Start new Claude session in selected project |
| `g` | Open lazygit in project directory |
| `G` | Open project on GitHub |
| `t` | Open terminal in project directory |
| `e` | Open editor in project directory |
| `?` | Toggle help |
| `q` / `Ctrl+C` | Quit |

## Configuration

Config file location: `~/.config/total-recall/config.toml`

```toml
[claude]
claude_dir = "~/.claude"      # Path to Claude data directory

[display]
preview_lines = 3             # Lines of conversation preview
date_format = "%m/%d %H:%M"   # Session date format
show_agent_sessions = true    # Show agent sub-sessions
```

## How It Works

total-recall scans your `~/.claude/projects/` directory for session files. Each project directory is named with an encoded path (e.g., `/home/user/myproject` becomes `-home-user-myproject`). Sessions are stored as JSONL files containing the conversation history.

When you select a session and press Enter, total-recall spawns a new terminal window and runs `claude --resume <session-id>` to continue the conversation.

## License

MIT
