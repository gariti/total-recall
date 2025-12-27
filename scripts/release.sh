#!/usr/bin/env bash
set -e

# Get version from Cargo.toml
VERSION=$(grep "^version" Cargo.toml | sed 's/.*"\(.*\)"/\1/')

echo "Releasing v$VERSION..."

# Build release
echo "Building release binary..."
cargo build --release

# Commit any changes (if any)
echo "Committing changes..."
git add -A
git commit -m "Release v$VERSION" || echo "No changes to commit"

# Tag the commit
echo "Tagging v$VERSION..."
git tag -a "v$VERSION" -m "Release v$VERSION"

# Push commit and tag
echo "Pushing to GitHub..."
git push origin main
git push origin "v$VERSION"

# Create GitHub release with binary
echo "Creating GitHub release..."
gh release create "v$VERSION" \
  --title "v$VERSION" \
  --generate-notes \
  target/release/total-recall

echo "Released v$VERSION!"
