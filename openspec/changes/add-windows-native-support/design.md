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
- A `service` module (in `bootstrap`, used by the CLI and the daemon-start resolver) with
  `start/stop/restart` — see D2.
- A `shell` lookup in `bootstrap::util` that returns the POSIX shell: `sh` on Unix, Git's `sh.exe` on
  Windows (same search order as `run-hook.cmd`: `Program Files\Git`, `Program Files (x86)\Git`, `PATH`).
  A miss is an error naming Git for Windows, never a silent skip.

### D2. Daemon lifecycle: Task Scheduler logon task
`sensei start` on Windows registers a per-user scheduled task `sensei-senseid` (re-registered on every
start, so it follows the binary after an upgrade) and runs it. Its trigger is the user's own logon and it
runs with their interactive token (`LeastPrivilege`), which lets a standard user register it. It sits at
the root of the Task Scheduler library: creating a folder (`sensei\senseid`, as first planned) needs an
administrator. The task runs `senseid start`, which spawns the daemon with no console window and exits,
so the task finishes at once; `ExecutionTimeLimit` is unlimited regardless, since the 72-hour default
ends whatever a task still runs. The definition is passed to `schtasks /Create /XML` as UTF-16.

`sensei stop` deletes the task (so the daemon does not return at the next sign-in, as `brew services
stop` does) and then runs `senseid stop`, which asks the daemon over `POST /stop` — the path every
platform already tries. If the daemon does not answer, the fallback ends it by pid: SIGTERM on Unix, a
terminate on Windows. Daemon status stays the direct `/health` probe on every platform, so the module has
no `status`. Rejected: a Windows Service — it runs under a service account, so the user's Credential
Manager entries and `~/.sensei` are not the ones the CLI and app see.

### D3. Postgres in Docker
The prerequisite provider checks, in order: Docker CLI present, Docker engine reachable, container
`sensei-pg` exists, container running, port 5432 answering. The remedy creates or starts it:
`docker run -d --name sensei-pg --restart unless-stopped -p 5432:5432 -v sensei-pgdata:/var/lib/postgresql/data -e POSTGRES_PASSWORD=… pgvector/pgvector:pg17`.
The daemon treats it as an external Postgres: it connects and applies the schema; it does not start or
stop the container.

**Credentials come from `.env` (user, 2026-11-19).** Today the daemon connects as the OS user with no
password (`postgres://localhost/<db>`, `bootstrap/src/database.rs`), which a container does not accept.
`~/.sensei/.env` holds the standard libpq names `PGUSER` and `PGPASSWORD`. The daemon, the CLI and the app
put them into their own process environment at startup (`credentials::load_into_env`) instead of into a
connection URL, because every client sensei uses reads them from there: libpq's `psql`/`createdb`/
`pg_isready`, and sqlx — the daemon's pool and dbd-core's deploy — whenever the URL carries no
credentials. The URLs stay credential-free, so the password cannot reach a log or an error through one. The
same values create the container (`POSTGRES_USER`/`POSTGRES_PASSWORD`, handed to `docker run` through its
environment, never its command line). Rules:
- The file is optional everywhere; without it, macOS/Linux keep connecting as the OS user, unchanged.
- On Windows a missing file, or a missing `PGUSER` or `PGPASSWORD`, fails the database check with a remedy
  naming the file and the keys. Nothing is generated or defaulted. (`PGUSER` is required too: without it
  the container's user would be `postgres` while clients connect as the OS user.)
- Real process environment variables take precedence over the file.
- The file is parsed literally (`KEY=VALUE`, optional `export` and quotes, no `$VAR` expansion, which would
  corrupt a password), and a parse error names the line number, never its text. The password is never logged.

**The Postgres tools run in the container (user, 2026-11-20).** The database setup and its check shell out
to `psql`, `createdb` and `pg_isready`, which a Windows host does not have. On Windows they run inside
`sensei-pg` (`docker exec -e PGUSER -e PGPASSWORD sensei-pg <tool>`), chosen in one place,
`pg_container::pg_tool`; macOS and Linux keep the host's tools. The pre-upgrade backup now reads
`\copy … to stdout` and sensei writes the CSV itself, so the file lands on the host on every platform.

**Components.** The health contract's five components stay as they are (the app reads them): Docker and
the container are checked under Postgres, the credentials under Database, and Git for Windows under
Sensei (its hooks run under Git's `sh`).

Docker Desktop's licence terms apply to large companies; Rancher Desktop and Podman
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
