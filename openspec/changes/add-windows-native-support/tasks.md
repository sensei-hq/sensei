## 1. Windows builds in CI (slice 1)
- [x] 1.1 Add a `windows-latest` job to the develop CI: `cargo check --workspace --target x86_64-pc-windows-msvc`
  (and clippy); record the failures it reports as the compile-fix list
- [x] 1.2 `home_dir()` resolves the profile directory on Windows; still errors when no home is set (test both)
- [x] 1.3 `shutdown_signal()` completes on Windows console control events (Windows test alongside the SIGTERM test)
- [x] 1.4 Free disk via `sysinfo`, replacing `libc::statvfs` in `db/pg_store/relay.rs` (existing tests green)
- [x] 1.5 Bootstrap log path from `std::env::temp_dir()` in `app/src-tauri` (`lib.rs`, `flog.rs`)
- [x] 1.6 Remaining items from 1.1 until the Windows check is green (4 rounds: Unix-only script/symlink tests
  gated, exec-bit check split; green on PR #507)
- [ ] 1.7 Make the `windows` job a required check on develop/main (branch protection — the user's call)
- [x] 1.8 Extend the Windows job to `app/src-tauri` (its own workspace; not covered by 1.1): frontend build,
  clippy and tests

## 2. Runtime on Windows (slice 2, depends on 1)
- [x] 2.1 POSIX shell lookup in `bootstrap::util` (Git `sh.exe` on Windows; a miss names Git for Windows)
- [x] 2.2 `metrics/coverage.rs` and other `sh -c` call sites use it (the only production one; the rest are tests)
- [x] 2.3 Process start time for `.in_use` matching on Windows — decided (user, 2026-11-20): write `{"pid":N}`
  with no start time. Claude Code keeps a separate `procStartFt` on Windows whose format the Linux build does not
  show, and its reader accepts a marker without one. Its Linux build reads `/proc/<pid>/stat`, not `ps`: #509
- [x] 2.4 `/Applications` probe in `assistants/mcp_file.rs` — no change: it is false on Windows and detection
  falls through to `bin_names` on PATH (`cursor`, `code`, `zed`, …)
- [ ] 2.6 Windows config paths for the MCP-file assistants (`config_rel` is macOS-shaped for Claude Desktop, and
  Zed / VS Code keep settings elsewhere on Windows) — verify each app's real Windows location before changing
- [x] 2.5 Run the no-DB tests on the Windows CI job (decided, user 2026-11-20: GitHub's Windows runners cannot
  start the pgvector service container, so the DB-coupled suite stays on Linux): every crate but senseid, plus
  senseid's pure modules via `scripts/senseid-pure-coverage.sh --test-only` (same selection and guards). Green
  on PR #510 after 3 rounds: Unix-only signal/script/permission tests gated, two tests' path expectations fixed

## 3. Daemon lifecycle (slice 3, depends on 1)
- [x] 3.1 `service` module with start/stop/restart: Homebrew services on macOS/Linux (current behaviour,
  tests unchanged), Task Scheduler logon task `sensei-senseid` on Windows. No `status`: daemon status
  stays the direct `/health` probe (D2)
- [x] 3.2 `sensei start|stop|restart` and the daemon-start resolver call it; `senseid start|stop` use the
  shared `util::pid_alive`/`util::terminate` instead of `kill`, and the daemon starts with no console
  window on Windows
- [ ] 3.3 Manual check on a Windows host: start, sign out/in (daemon back), stop (graceful shutdown logged)

## 4. Prerequisite checker (slice 4, depends on 3)
- [x] 4.1 Git for Windows checker (under the sensei component) + `winget install Git.Git` resolver
- [x] 4.0 Read `~/.sensei/.env` (`PGUSER`, `PGPASSWORD`; env vars win) into the process environment at
  startup of the daemon, CLI and app — every client reads them there, so no URL carries them; macOS/Linux
  unchanged without the file; parse errors name the line, never its text (tests for each)
- [x] 4.2 Docker CLI/engine + `sensei-pg` container checker; resolver creates or starts the container and
  waits for `pg_isready`; Postgres tools run in the container via `docker exec` (`pg_container::pg_tool`);
  the pre-upgrade backup streams through stdout to a host file
- [x] 4.3 Ollama checker + `winget install Ollama.Ollama` resolver; daemon and database checkers (database
  behind the credentials check); daemon-start remedies use the platform's service commands
- [x] 4.4 Replace the stub's "not yet implemented" tests with per-component tests against fake probes
- [ ] 4.5 On a Windows host: `sensei doctor` on a fresh machine lists each missing component; `--fix`
  creates `sensei-pg`, sets up the database and starts the daemon
- [ ] 4.6 The daemon-start resolver's "binary missing" remedy still says `brew install` on Windows (slice 5
  defines the Windows install)

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
