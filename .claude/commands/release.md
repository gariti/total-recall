---
description: Full release workflow with checks, tests, code review, fixes, and GitHub release
allowed-tools: Bash, Read, Edit, Write, Grep, Glob, Task, AskUserQuestion, TodoWrite
---

# Release Workflow

You are executing the release workflow for this Rust project. Follow these steps carefully.

## Step 1: Set up tracking

Use TodoWrite to create a todo list with these items:
1. Run cargo check
2. Run cargo test
3. Code review changes
4. Fix any issues
5. Summarize changes
6. Choose version increment
7. Update version and release

## Step 2: Run checks

Run `cargo check` to verify the code compiles. If there are errors, fix them before proceeding.

## Step 3: Run tests

Run `cargo test` to ensure all tests pass. If tests fail, fix them before proceeding.

## Step 4: Code review

Use the Task tool with subagent_type="pr-review-toolkit:code-reviewer" to review the current changes. The agent should review unstaged changes (run `git diff` first to see what's changed).

## Step 5: Fix issues

If any issues were found in steps 2-4:
- Fix compile errors
- Fix failing tests
- Address code review feedback

Continue automatically after fixing - do not ask for confirmation.

## Step 6: Summarize

Provide a brief summary of:
- What changes are included in this release
- Any fixes you made during the review process
- Current state of the codebase (all checks passing)

## Step 7: Ask about version

Read the current version from Cargo.toml. The version line looks like: `version = "X.Y.Z"`

Use AskUserQuestion to ask the user which version increment they want:
- **Patch** (X.Y.Z → X.Y.Z+1): Bug fixes, minor changes
- **Minor** (X.Y.Z → X.Y+1.0): New features, backwards compatible
- **Major** (X.Y.Z → X+1.0.0): Breaking changes

Include the current version and what each option would result in.

If the user declines or says no, stop the workflow gracefully.

## Step 8: Update version and release

1. Edit Cargo.toml to update the version number to the new version
2. Run `./scripts/release.sh` to:
   - Build the release binary
   - Commit the changes
   - Create a git tag
   - Push to GitHub
   - Create a GitHub release with the binary

## Completion

Report the final status:
- New version number
- GitHub release URL (from the release script output)
- Confirm the release was successful
