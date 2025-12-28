# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

**total-recall** is a Rust-based TUI (Terminal User Interface) for browsing and resuming Claude Code conversations across all your projects. "Get your ass to Claude."

## Build Commands

```bash
# Build debug version
cargo build

# Build optimized release
cargo build --release

# Run directly
cargo run

# Run with debug logging
cargo run -- --debug

# Run with custom Claude directory
cargo run -- --claude-dir /path/to/.claude

# Check for errors without building
cargo check
```

## Architecture

```
src/
├── main.rs              # CLI argument parsing, logging setup
├── app.rs               # Main TUI event loop, screen management
├── config.rs            # TOML configuration with defaults
├── screens/             # TUI screens (each implements Screen trait)
│   ├── browser.rs       # Project/session browser (main screen)
│   ├── search.rs        # Full-text search (TODO)
│   ├── preview.rs       # Conversation preview (TODO)
│   └── stats.rs         # Usage statistics (TODO)
├── services/            # Backend service layer
│   ├── session_store.rs # JSONL parsing, session discovery
│   ├── clipboard.rs     # System clipboard integration
│   ├── search_index.rs  # Full-text search (TODO)
│   └── metadata_store.rs# Tags/favorites SQLite storage (TODO)
├── models/              # Data structures
│   ├── session.rs       # Session summary
│   ├── message.rs       # JSONL message types
│   └── project.rs       # Project grouping
└── utils/               # Utility functions
    └── mod.rs           # Path encoding/decoding
```

### Key Design Patterns

- **Screen Trait**: All TUI screens implement `async_trait Screen` with `draw()` and `handle_key()`
- **Service Layer**: Backend services wrapped in `Arc` for shared ownership
- **Async/Await**: Uses `tokio` runtime for async operations

### Data Flow

1. **SessionStore** scans `~/.claude/projects/` for JSONL session files
2. Projects and sessions are parsed and cached
3. **BrowserScreen** displays dual-pane list (projects | sessions)
4. User selects session, presses Enter to copy resume command to clipboard
5. User pastes command in terminal to resume session

## Configuration

Default config location: `~/.config/total-recall/config.toml`

```toml
[claude]
claude_dir = "~/.claude"  # Path to Claude directory

[display]
preview_lines = 3         # Lines of preview text
date_format = "%m/%d %H:%M"
show_agent_sessions = true
```

## Claude Code Data Structures

Sessions are stored in `~/.claude/projects/<encoded-path>/`:
- Each project directory is encoded: `/home/user/project` → `-home-user-project`
- Each session is a `.jsonl` file named by session UUID
- Messages are JSON objects, one per line

## Keybindings

- `j/k` or `↑/↓` - Navigate lists
- `h/l` or `←/→` - Switch panes
- `Enter` or `y` - Copy resume command to clipboard
- `q` or `Ctrl+C` - Quit

## Development Workflow

**IMPORTANT: Always verify code compiles after making changes.**

- After any code modifications, run `cargo check` to verify the code compiles without errors
- Fix any compilation errors before considering the task complete
- Run `cargo check` before committing
- Use `cargo run` for quick testing
- Build release with `cargo build --release`
