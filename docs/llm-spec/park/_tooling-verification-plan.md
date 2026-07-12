# SPEC — Sensei tooling correctness, anti-drift, upgrade hardening, live verification
Owner: autonomous run · Priority: **TOP — before all remaining sweep/Dōjō/UI work** (Jerry, 2026-07-12)

## Why (root causes, all dogfooded live)
The assistant kept re-deriving wrong conclusions because sensei's own tools didn't work here:
- **Stale daemon:** running daemon was **v0.2.29**, repo **v0.2.39** — 10 releases. `make bump` never installs; nobody ran `make install`. → all MCP calls hit old code.
- **MCP↔daemon param DRIFT:** the MCP proxy sends params the daemon rejects — `get_layered_context` sends `project_id=<NAME>` (daemon wants a UUID → 400); `get_rules` sends `folder=<mcp-process-cwd>` (wrong folder). Each side is unit-green; the **seam is untested** (`knowledge_api.rs:45` tests the daemon with a UUID — the very contract the proxy violates).
- **Stale processes after upgrade:** the sensei MCP is a **long-lived stdio subprocess owned by Claude Code**, not a brew service. `make install` restarts the daemon (launchd) but leaves the old `sensei-mcp` process in memory → tools keep running old code until the MCP connection is reloaded.
- **Data is fine:** 558,904 nodes; `sensei` repo → project `ff1ccea2` (3395 fns). Scanner picked the right context. Also a **duplicate empty** `sensei` project `2efd4ecf` (0 nodes) that `list_projects` surfaces.

## Goals
1. MCP tools return GENUINE DB-backed results for the folder/project you're working in.
2. The MCP↔daemon contract **cannot silently drift** — enforced by tests.
3. Upgrades leave **no stale processes** — daemon + MCP both current after an install.
4. The **full cycle is live-verified on 3 first-class example repos**, repeatably.

## First-class example repos (live test corpus)
Diverse, real, already-indexed (all in `list_projects`):
- **sensei** `ff1ccea2` — monorepo (rust + sveltekit + tauri + postgres). `/Users/Jerry/Developer/sensei-hq/sensei`
- **rokkit** `86066f90` — svelte UI library. `/Users/Jerry/Developer/rokkit`
- **dbd-rs** `6b95f063` — rust CLI. `/Users/Jerry/Developer/dbd-rs`
(kavach `71f7a319` is a 4th if we want a 2nd auth/TS repo.) The full-cycle harness (F) runs against these.

## Workstreams (sequenced; this whole block is P0)
### A. Resolution correctness  — ⏳ BUILDING (agent aa37a598)
Daemon `get_context`/`get_rules` accept `project` (name) → resolve name→uuid daemon-side (reuse the `resolve_project_uuid` used by `/commands`); MCP proxy sends a value the daemon accepts; keep `project_id`/`folder` working. + the first MCP↔daemon integration test (must go red on proxy-sends-name → daemon-400).

### B. Anti-drift contract coverage
A test that, for **every** MCP tool, boots the daemon (in-process axum app / the `tests/` harness against `sensei_test`) with a seeded project, invokes the MCP proxy's `handle_call_tool` (or the exact HTTP it builds), and asserts a non-error, genuinely-shaped result. Enumerated + ideally table-driven over the tool list so a NEW tool or a changed daemon param FAILS the suite. This is the regression guard against future drift.

### C. Folder→project workflow (the "which project am I on" fix)
- `find_projects` (or `list_projects?under=<path>`): projects whose `abs_path` is under a folder (default cwd) → "find projects under current folder".
- `use_project`/pin: write the active project to a pin file `~/.sensei/active-project` (name+id) that the MCP reads per-call as the default when cwd doesn't resolve → "I'm working on sensei" works regardless of cwd. (MCP process cwd is fixed, so a pin file is the robust mechanism.)

### D. Upgrade / install hardening
- **bump ⇒ install (release):** a `make ship` = `make bump && make install`; the autonomous loop runs `make install` (release) after every milestone bump. [[feedback_bump_then_install]]
- **no stale processes (VERIFIED behavior):** the sensei MCP is a long-lived stdio subprocess that is a CHILD of the `claude` session (PID 83430 ← claude 83408). `make install` overlays the brew binary (the wrapper `bin/sensei-mcp.sh` execs `/opt/homebrew/bin/sensei-mcp`, now 0.2.39) but the RUNNING process keeps the old in-memory binary. TESTED: `kill`ing the subprocess does NOT auto-respawn in-session — Claude Code drops the MCP connection (all tools go unavailable). So a running session's MCP needs a CLIENT-SIDE reconnect; install cannot self-heal it. Also: the Claude Code sensei PLUGIN is pinned at **v0.2.29** in `~/.claude/plugins/cache/` (hooks + wrapper stale). → The correct post-release refresh is **`claude plugin update sensei`** (updates the plugin cache 0.2.29→current AND reconnects the MCP) or a `/mcp` reconnect. `make install`/`make ship` should `pkill sensei-mcp` (so the next session/reconnect gets the fresh binary) + PRINT a reminder to run `claude plugin update sensei`.
- **version-change worker:** on daemon boot, compare running binary version vs a stored `last_analyzed_version`; if changed, (a) enqueue a full rescan + re-analysis so the graph/memories rebuild against the new binary, and (b) trigger the **assistant upgrade** (below) so each connected assistant's plugin/MCP refreshes.
- **⭐ Assistant upgrade/refresh (Jerry's fix — put `claude plugin update sensei` in the CLI/assistant layer):** the abstraction already exists — `crates/senseid/src/assistants/` has an `Assistant` trait + `configure()` that runs `claude plugin **install**` (`claude_code.rs`, verified via `installed_plugins.json`), exposed at `/api/assistants/{detect,configure,remove}` + `sensei` CLI. ADD an **`upgrade()`** trait method: `ClaudeCodeAssistant::upgrade` → `claude plugin update sensei` (mirror the install + verify pattern) + note the MCP reconnect; Zed/Cursor → re-read their file-based MCP config (largely no-op). Expose as `/api/assistants/upgrade` (fan out over detected assistants) + a `sensei upgrade` CLI subcommand. Auto-invoke from the version-change worker after an install so a sensei upgrade refreshes EVERY assistant's plugin/MCP with zero manual steps → the stale-plugin/stale-MCP class of bug never recurs on delivery. NOTE: an in-session MCP still needs the client to reconnect, but `claude plugin update` is the correct trigger and future sessions pick up the new binary automatically.

### E. Dedup the duplicate empty project
Merge/prune `2efd4ecf` (empty "sensei") into `ff1ccea2` (the real index); prevent name-duplicate projects (scan-reconcile guard).

### F. Live full-cycle verification on the 3 repos
A repeatable check (script or db-gated test) that, per repo, resolves the project **from its folder**, runs get_project_summary / search / get_layered_context / get_rules / get_patterns / get_project_conventions / get_duplicates, and asserts genuine NON-EMPTY DB-backed results. Run it after install + MCP reload. This is the "is the cycle working" gate, first-class going forward.

## Exit → resume autopilot
When A–F are done + the 3-repo live check is green, RESUME the original queue full-steam (sweep A–G: Atlas/logs-UI/traceability/impact/icon-asset-serve/benchmarks/TDD-gate; Dōjō console + supabase + protocol consolidation). Standing rules unchanged: default-and-proceed, install-after-bump, visually verify UI via `make test-app-e2e`.

## Status
- [x] Daemon current (release v0.2.39 live) · [x] Diagnosis · [x] Spec
- [ ] A resolution+first integration test (building) · [ ] B contract coverage · [ ] C find/pin · [ ] D install-hardening+worker · [ ] E dedup · [ ] F 3-repo live check · [ ] resume autopilot
