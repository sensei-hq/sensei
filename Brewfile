# Install Sensei and its prerequisites via Homebrew.
# Usage: brew bundle --file=Brewfile
# Safe to re-run — already-installed items are skipped.

tap "sensei-hq/tap", "https://github.com/sensei-hq/homebrew-tap"

brew "postgresql@17"                # Storage engine
brew "ollama"                       # Local AI inference (embeddings, summarisation)
brew "sensei-hq/tap/sensei"        # CLI + MCP bridge + background daemon
cask "sensei-hq/tap/sensei-app"    # Desktop app (optional)
