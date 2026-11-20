# Design: Native Windows support

## Context
Every platform assumption today is macOS-first with Linux following: Homebrew installs the binaries and
Postgres, `brew services` (launchd/systemd) keeps the daemon up and stops it with SIGTERM, hooks are bash,
and `HOME` names the user's directory. Windows has none of these. The decisions in `proposal.md` remove
the two largest risks (pgvector on Windows, certificate lead time) from this change.

## Goals / Non-Goals
- Goals: a Windows user installs the desktop app and the CLI tools from a GitHub release, runs
  `sensei doctor --fix`, and gets a working daemon, MCP server and hooks.
- Non-Goals: signed builds, a package-manager listing, a Postgres the daemon owns on Windows.

## Decisions

### D1. One platform seam per concern, not scattered `cfg`
Each Unix-only behaviour gets one function with a `cfg(unix)`/`cfg(windows)` body, in the crate that
already owns the concern, and callers stay platform-free:
- `bootstrap::config::home_dir()` — `HOME` on Unix, the profile directory on Windows (`dirs`/`USERPROFILE`).
  It still errors when neither is set; it never falls back to a temp dir.
- `senseid::shutdown::shutdown_signal()` — SIGINT/SIGTERM on Unix; Ctrl-C, Ctrl-Break, close and
  shutdown console events on Windows.
- Free disk via `sysinfo` (already a dependency of `bootstrap` and the app) on every platform, replacing
  the `libc::statvfs` call.
- A `service` module (in `bootstrap`, used by the CLI) with `start/stop/restart/status` — see D2.
- A `shell` lookup in `bootstrap::util` that returns the POSIX shell: `sh` on Unix, Git's `sh.exe` on
  Windows (same search order as `run-hook.cmd`: `Program Files\Git`, `Program Files (x86)\Git`, `PATH`).
  A miss is an error naming Git for Windows, never a silent skip.

### D2. Daemon lifecycle: Task Scheduler logon task
`sensei start` on Windows registers (once) and runs a per-user scheduled task `sensei\senseid`,
triggered at logon, running `senseid serve` hidden. `stop` asks the daemon over `POST /stop` first (the
path every platform already tries), then ends the task. Rejected: a Windows Service — it runs under a
service account, so the user's Credential Manager entries and `~/.sensei` are not the ones the CLI and
app see.

### D3. Postgres in Docker
The prerequisite provider checks, in order: Docker CLI present, Docker engine reachable, container
`sensei-pg` exists, container running, port 5432 answering. The remedy creates or starts it:
`docker run -d --name sensei-pg --restart unless-stopped -p 5432:5432 -v sensei-pgdata:/var/lib/postgresql/data -e POSTGRES_PASSWORD=… pgvector/pgvector:pg17`.
The daemon treats it as an external Postgres: it connects and applies the schema; it does not start or
stop the container.

**Credentials come from `.env` (user, 2026-11-19).** Today the daemon connects as the OS user with no
password (`postgres://localhost/<db>`, `bootstrap/src/database.rs`), which a container does not accept.
The daemon reads `~/.sensei/.env` and takes the standard libpq names `PGUSER` and `PGPASSWORD` from it
(so `psql` and `dbd` read the same values), building the connection URL in the one place that builds it
today. The same values feed the container remedy (`POSTGRES_USER`/`POSTGRES_PASSWORD`). Rules:
- The file is optional everywhere; without it, macOS/Linux keep connecting as the OS user, unchanged.
- On Windows a missing file, or a missing `PGPASSWORD` in it, fails the database check with a remedy
  naming the file and the keys. Nothing is generated or defaulted.
- Real process environment variables take precedence over the file.
- The password is never logged, and URLs in logs and errors are redacted. Docker Desktop's licence terms apply to large companies; Rancher Desktop and Podman
run the same image and pass the same checks (they answer `docker` CLI calls).

### D4. Distribution: GitHub release assets, unsigned
`release.yml` adds a `windows-latest` job for `x86_64-pc-windows-msvc`: `sensei-windows-x86_64.zip`
(`senseid.exe`, `sensei.exe`, `sensei-mcp.exe`) and the Tauri NSIS installer, which also places the three
binaries and adds them to the user `PATH`. No signing step; the README states the SmartScreen prompt.

## Risks / Trade-offs
- **Defender false positives on unsigned Rust binaries** → submit to Microsoft if testers hit it; signing
  removes this later.
- **`ps -o lstart=` matching Claude Code's `.in_use` markers** → on Windows the start time must be read
  the way Claude Code writes it there; verify the marker format on a Windows host before choosing the API.
- **Path forms in hooks** (`C:\…` vs `/c/…`) under Git Bash → covered by the hook end-to-end task.
- **Unknown compile breaks** → the Windows `cargo check` job is the first task, so the list is found, not
  guessed.

## Open Questions
- None blocking. (The container password was settled: `.env`, see D3.)
