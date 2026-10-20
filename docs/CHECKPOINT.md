# Checkpoint — 2026-10-19

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Waves 1 and 2 are done; wave 3 is next.

**Done:**
- **Wave 1 (Settings → Projects):** #271 and #269 closed with live evidence. Also fixed: role null-clear
  (e777d692) and the watcher rescan demoting modules (ffc79fc6).
- **Wave 3 so far:** #104, #185, #186, #153, #154, #177, #157, #280, #213, #135, #179, #181 and #96
  closed. #281 split out; #283 and #284 filed from #96.
- **Dependencies:**
  - dbd 0.27.3 everywhere (2d77d771). The live DB diffs clean.
  - rustls 0.23.46 (c11e19d2, #172).
  - #190, #258, #122 and #139 carry status comments.

**Remaining:**
- Wave 2 is decided:
  - closed #141, #230, #90
  - filed #277, #278, and deferred epic #279 (the Benchmark runner, re-designed on transcript history)
  - `deferred` label on #50 and #150
  - Rust is the bar for every language (#275)
- #144 moved to wave 3 (the project metrics view, not Settings).
- #190 moved up to wave 5: since dbd 0.27, a workspace (`modules:`) design cannot be imported until
  `project::survey` + `Workspace::load` land.
- Wave 0 leftovers: #240 (production-signed-in push), #184 (`sync_status`).
- After waves 0–8: the full re-index (wave 9), which restores the 130 projects whose checkouts are
  gone. Then language parity for Java, C# and Python (wave 10, #275, blocked first by #276).

**Next command:** `/sensei:session refocus`, then #283's last piece: `POST /api/auth/keychain/approve` (re-read the
persona's stored session inside `gateway_keys::allow_prompt`), plus an "Approve keychain access" button where the
app shows the forge standing. Then #285.

**Open questions:**
- Run `make dev-signing-cert` once (asks for your login password) so local installs keep their keychain and folder grants. #282 needs an Apple Developer account for releases.
- #194: drop the 145k/day `task_started`/`task_completed` rows from `public.logs`? That would also remove them from the Activity Logs screen.
- The orphaned Cloudflare build trigger fails every develop push. Delete it in the dashboard?
- 8 CodeQL `rust/cleartext-logging` alerts in test files need triage.

**Known-broken / pending:**
- dōjō sync fails with "keychain access needs approval" on this machine until the approve action lands, or you
  sign in again (a sign-in may prompt). `make dev-signing-cert` keeps it from recurring after each rebuild.
- `/api/projects/{id}/icon` 404s for projects whose checkout is gone. The glyph falls back to kanji.
- Dependabot reports 0 alerts although the lockfiles are vulnerable (#172).
- The committed `database/design.dbml` is stale (pre-existing).
