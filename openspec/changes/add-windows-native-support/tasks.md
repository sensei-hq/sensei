## 1. Windows builds in CI (slice 1)
- [ ] 1.1 Add a `windows-latest` job to the develop CI: `cargo check --workspace --target x86_64-pc-windows-msvc`
  (and clippy); record the failures it reports as the compile-fix list
- [ ] 1.2 `home_dir()` resolves the profile directory on Windows; still errors when no home is set (test both)
- [ ] 1.3 `shutdown_signal()` completes on Windows console control events (Windows test alongside the SIGTERM test)
- [ ] 1.4 Free disk via `sysinfo`, replacing `libc::statvfs` in `db/pg_store/relay.rs` (existing tests green)
- [ ] 1.5 Bootstrap log path from `std::env::temp_dir()` in `app/src-tauri` (`lib.rs`, `flog.rs`)
- [ ] 1.6 Remaining items from 1.1 until the Windows check is green; make the job required

## 2. Runtime on Windows (slice 2, depends on 1)
- [ ] 2.1 POSIX shell lookup in `bootstrap::util` (Git `sh.exe` on Windows; a miss names Git for Windows)
- [ ] 2.2 `metrics/coverage.rs` and other `sh -c` call sites use it
- [ ] 2.3 Process start time for `.in_use` matching on Windows (verify Claude Code's marker format there first)
- [ ] 2.4 `/Applications` probe in `assistants/mcp_file.rs` gains the Windows install locations
- [ ] 2.5 Run the full senseid suite on the Windows CI job (DB tests against a `pgvector/pgvector:pg17` service)

## 3. Daemon lifecycle (slice 3, depends on 1)
- [ ] 3.1 `service` module with start/stop/restart/status: Homebrew services on macOS/Linux (current behaviour,
  tests unchanged), Task Scheduler logon task on Windows
- [ ] 3.2 `sensei start|stop|restart` and `senseid stop` call it; the `kill <pid>` fallback is Unix-only
- [ ] 3.3 Manual check on a Windows host: start, sign out/in (daemon back), stop (graceful shutdown logged)

## 4. Prerequisite checker (slice 4, depends on 3)
- [ ] 4.1 Git for Windows checker + `winget install Git.Git` remedy
- [ ] 4.0 Read `~/.sensei/.env` (`PGUSER`, `PGPASSWORD`; env vars win) where the connection URL is built;
  macOS/Linux unchanged without the file; password redacted in logs/errors (tests for each)
- [ ] 4.2 Docker CLI/engine checkers + `sensei-pg` container checker; remedy creates or starts the container
- [ ] 4.3 Ollama checker + `winget install Ollama.Ollama` remedy; daemon and database checkers
- [ ] 4.4 Replace the stub's "not yet implemented" tests with per-component tests against fake probes

## 5. Distribution (slice 5, depends on 2–4)
- [ ] 5.1 `release.yml`: `windows-latest` job building `sensei-windows-x86_64.zip`, attached to the release
- [ ] 5.2 Tauri NSIS installer (unsigned), shipping the three binaries on the user `PATH`; Windows fallback for
  `titleBarStyle: Overlay` / `macOSPrivateApi`
- [ ] 5.3 Hooks end to end on Windows through `run-hook.cmd` → Git Bash (session-start, forward, nudge, pre-compact)
- [ ] 5.4 README: Windows install (Git for Windows, Docker, the installer, SmartScreen prompt, `sensei doctor --fix`)

## 6. Verification
- [ ] 6.1 Clean Windows 11 VM: follow the README only, end with `sensei doctor` all green, a workspace indexed,
  and a Claude Code session captured with hooks firing
- [ ] 6.2 `openspec validate add-windows-native-support --strict`
