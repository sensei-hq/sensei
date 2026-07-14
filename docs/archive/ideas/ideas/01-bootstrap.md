# Bootstrap

Bootstrap is the first thing you see when Sensei opens. It checks every dependency needed to run, installs anything missing, and starts the background services. On a healthy system this takes under two seconds. On a fresh install, it walks you through everything.

---

## The 6 gates

Bootstrap runs six checks in order. Each gate follows the same cycle: detect whether the component is present, install or fix it if not, then verify it's working.

### 1. Homebrew

The package manager that handles all other installations on macOS. If Homebrew is already installed, this gate passes instantly. If missing, Sensei shows the install command and waits for you to run it in your terminal (Homebrew requires terminal interaction). Once Sensei detects `/opt/homebrew/bin/brew`, it advances automatically.

### 2. PostgreSQL

The database. If not installed, Sensei installs it via Homebrew and starts the service. If already present, it verifies the service is running and the port is reachable. No user action needed -- Sensei handles install, start, and verification automatically.

### 3. Ollama

The local inference engine that powers embeddings, classification, and reasoning. Sensei installs it via Homebrew, starts the service, and pulls the models your hardware can support. Model downloads can take time, so this gate can be skipped -- you can pull models later from settings. Sensei detects your system's RAM and GPU to recommend the right model tier.

### 4. Sensei components

The daemon, MCP server, and CLI. These are the Rust binaries that do the actual work -- indexing code, serving tools to your AI assistant, and running the API. Sensei installs or upgrades them via its Homebrew tap. If your installed version doesn't match the desktop app version, Sensei prompts an upgrade.

### 5. Database

Creates the Sensei database if it doesn't exist, installs required extensions, and runs schema migrations to make sure the data model is current. This is a one-time operation on first install; on subsequent launches it just verifies the schema version matches.

### 6. Daemon

Starts the senseid process and waits for its health endpoint to respond. Once the daemon is up, Sensei has everything it needs. On a cold start where the database was just created, the daemon may need a restart after migrations complete -- bootstrap handles this automatically.

---

## What the user sees

The bootstrap screen shows a list of all six gates with status indicators for each. Every gate displays its name, version (when known), and current state.

| State | Indicator | Meaning |
|-------|-----------|---------|
| Detecting | Pulsing amber | Checking if installed and running |
| Installing | Progress bar with size | Homebrew install in progress |
| Pulling | Progress bar with model size | Downloading a local model |
| Starting | Pulsing amber | Service starting up |
| Upgrading | Progress bar with version numbers | Upgrading to match desktop version |
| Ready | Solid jade dot with version | Healthy |
| Failed | Amber dot with error message | Needs attention |
| Skipped | Grey dot | User chose to skip (e.g. Ollama) |

An activity log scrolls beneath the gate list showing what's happening in real time. A privacy note at the bottom confirms that nothing leaves your machine.

---

## Repeat launches

Every time you open Sensei, bootstrap runs a health check. If all six gates pass -- which takes under two seconds on a healthy system -- you go straight to the observatory (or the setup wizard if it's your first time). The bootstrap screen only stays visible when something needs fixing.

---

## Upgrades

When the desktop app updates, the bundled VERSION may be newer than the installed Sensei components. Bootstrap detects this mismatch at gate 4 and prompts you to upgrade via Homebrew. After the upgrade, it re-runs migrations (gate 5) and restarts the daemon (gate 6). It also checks whether your configured AI assistants need their extensions updated -- if so, it re-pushes skills, hooks, and commands silently. On a healthy system, the entire upgrade path is invisible.

---

## Error recovery

When a gate fails, it expands to show the error message and what you can do about it. Most gates have a retry button that re-runs the check. The diagnostic log is accessible from the bootstrap screen for deeper troubleshooting.

Non-critical gates can be skipped. Ollama is the primary example -- Sensei works without local inference, just with reduced capabilities (no local embeddings or classification). You can come back to it later from settings.

Critical gates (Homebrew, PostgreSQL, database, daemon) must pass before Sensei can proceed. If Homebrew is missing, Sensei shows the install command with a copy button and polls until it detects the installation. For PostgreSQL or daemon failures, the error details and retry mechanism are usually enough to resolve the issue.

---

## Reference

- Mockups: `docs/mockups/lib/bootstrap.jsx`
- Implementation details: [design/01-app.md](../design/01-app.md)
