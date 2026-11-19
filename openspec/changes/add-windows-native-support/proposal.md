# Change: Native Windows support for consumers (#505)

## Why
sensei ships for macOS and Linux only. `release.yml` builds four targets (darwin arm64/x86_64, linux
arm64/x86_64), the desktop app is a macOS-only Homebrew cask, and the Windows prerequisite provider
(`crates/bootstrap/src/health/platforms/windows.rs`) is a stub that reports "not yet implemented" for
every component. A Windows user today can only run the Linux build under WSL2, without the desktop app.

## Decisions (user, 2026-11-19)
- **Postgres runs in Docker** (`pgvector/pgvector:pg17`) on Windows. No native Postgres install, no
  pgvector build for Windows. The daemon already connects to `localhost:5432`; the container's
  credentials reach the daemon through `~/.sensei/.env` (`PGUSER`, `PGPASSWORD`).
- **Unsigned test builds.** No Authenticode certificate for this change; testers accept the SmartScreen
  prompt. winget distribution waits for signing.
- **Git for Windows is a prerequisite.** Hooks stay bash, run through the existing
  `marketplace/plugins/sensei/hooks/run-hook.cmd` → Git Bash. The daemon's `sh -c` call sites use the
  same Git `sh.exe`.
- **The daemon runs from a per-user Task Scheduler logon task** (not a Windows Service), so Credential
  Manager entries and `~/.sensei` resolve as the signed-in user, as under launchd on macOS.

## What Changes
- CI gains a Windows `cargo check` (and test) job on develop, so Unix-only code cannot land unnoticed.
- Compile fixes: shutdown signals (`tokio::signal::unix` → Windows console control events), free disk
  (`libc::statvfs` → `sysinfo`), the `kill <pid>` stop fallback, `HOME` (→ the platform home dir), and
  the hardcoded `/tmp/sensei-bootstrap.log`.
- Runtime fixes for Unix-only shell-outs: `ps -o lstart=`, `sh -c`, `brew services`, `/Applications`.
- A service-control layer behind `sensei start|stop|restart`: Homebrew services on macOS/Linux, a Task
  Scheduler logon task on Windows.
- The Windows prerequisite provider becomes real: Git for Windows, Docker + the `sensei-pg` container,
  Ollama, the daemon, the database — each with a winget/docker remedy.
- `release.yml` gains a Windows job: an unsigned Tauri installer for the desktop app and a zip of
  `senseid`/`sensei`/`sensei-mcp`, attached to the GitHub release.
- The desktop app's macOS-only window options get a Windows fallback.

## Out of Scope
- Bundled or native Postgres, pgvector for Windows.
- Code signing, winget/Scoop manifests.
- Porting hooks off bash.
- Windows on ARM.

## Impact
- Specs: `windows-runtime`, `windows-prerequisites`, `windows-distribution` (all new).
- Code: `crates/senseid` (`shutdown.rs`, `main.rs`, `db/pg_store/relay.rs`, `assistants/`,
  `tasks/handlers/metrics/coverage.rs`), `crates/bootstrap` (`config.rs`, `util.rs`,
  `health/platforms/windows.rs`), `crates/cli` (start/stop/restart), `app/src-tauri` (log path, window
  config), `.github/workflows/` (CI + `release.yml`), `README.md` (Windows install section).
- Estimate: 3–4.5 engineer-weeks across five shippable slices (see `tasks.md`).
