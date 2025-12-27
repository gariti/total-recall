---
description: Open a new terminal with cargo run for manual testing
allowed-tools: Bash
---

# Validate

Open a new terminal with `cargo run` so the user can manually test the application:

```bash
wezterm start --always-new-process --cwd '/home/garrett/Projects/total-recall' -- cargo run &>/dev/null &
```

Run this command and confirm to the user that the terminal has been opened for testing.
