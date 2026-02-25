# Ghostty CLI Reference

## `ghostty +new-window` (D-Bus IPC) — BROKEN in 1.3.0-dev

Sends a D-Bus message to a running ghostty instance to open a new window. The CLI process exits immediately.

**In ghostty 1.3.0-dev, `-e` is silently ignored.** The window opens with the default shell regardless of what command you pass. This was confirmed empirically — the daemon log shows `started subcommand path=/bin/sh` even when `-e tmux` is specified.

Only `--class` actually works. Everything else is silently dropped:
- `-e <command>` -- **silently ignored** (opens default shell)
- `--working-directory=PATH` -- **silently ignored** (see [ghostty#9508](https://github.com/ghostty-org/ghostty/discussions/9508))
- `.env()` calls -- env vars set on CLI process, NOT transmitted via D-Bus
- Any other config flags

## `ghostty` (new process, no `+`) — USE THIS

Spawns a brand new ghostty process. Supports ALL config keys as `--key=value` flags. `-e` works. `--working-directory` works. Slower startup (full GPU/font/Wayland init) but everything actually functions.

When `gtk-single-instance=true` is set AND no CLI args are present, it contacts the existing instance instead. But passing ANY arg (like `-e`) disables single-instance detection and spawns a new process.

## total-recall approach

Use plain `ghostty` with tmux for all spawned windows:

```
ghostty -e tmux new-session -A -s session-name -c /path/to/dir "command"
```

- `-e` runs the command instead of default shell
- tmux `-c` sets working directory
- tmux `-A` attaches to existing session if it exists
- tmux `-s` names the session for reattachment

## Tmux `;` separators via argv

Tmux interprets literal `;` in its argv as command separators:

```
ghostty -e tmux new-session -A -s name -c /path "command" \; set status on \; set status-left "text"
```

Each `\;` (or literal `;` when passed as a separate arg) starts a new tmux command within the same session. Used for configuring the tmux status bar inline.
