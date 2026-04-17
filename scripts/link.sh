#!/usr/bin/env bash
# (Re)create ~/.local/bin symlinks pointing to workspace target/release binaries.
# Run after `cargo build --release` or add to your build workflow.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="$REPO_ROOT/target/release"
DEST="${HOME}/.local/bin"

BINS=(senseid sensei sensei-mcp)

mkdir -p "$DEST"

for bin in "${BINS[@]}"; do
  src="$TARGET/$bin"
  if [[ ! -f "$src" ]]; then
    echo "skip: $bin (not built yet)"
    continue
  fi
  ln -sf "$src" "$DEST/$bin"
  echo "linked: $DEST/$bin -> $src"
done
