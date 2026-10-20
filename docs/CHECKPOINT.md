# Checkpoint — 2026-10-20

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Waves 1 and 2 done; wave 3 in progress.

**Done:**
- Wave 1: #271, #269. Wave 2 decided (#141, #230, #90 closed; #277, #278, deferred epic #279; #275).
- Wave 3: #104, #185, #186, #153, #154, #177, #157, #280, #213, #135, #179, #181, #96, #285 closed.
- #283 live: `running_since` + "running for", a per-pass limit ("still running after Ns"), a daemon that never
  prompts for the keychain, and an "Approve keychain access" action.
- dbd 0.27.3 (2d77d771), rustls 0.23.46 (c11e19d2).

**Remaining:**
- #283: verify the approve round trip after the user clicks it (then close). #133 slice 1 done; then #284,
  #286, #85, #144, #149, #278, dōjō (#132, #106, #191), code health (#199, #201, #139).
- #190 (wave 5): a `modules:` design cannot be imported until `project::survey` + `Workspace::load` land.
- Wave 0 leftovers: #240, #184. Wave 9: the full re-index. Wave 10: Java/C#/Python parity (#275, #276).
- #187 needs `make test-app-e2e-cold` (it stops postgres and ollama). #194 waits on a live failure.

**Next command:** `/sensei:session refocus`, then #133 slice 3b: `PageHeader` (it already takes `icon`; give it
`mode` and pass each screen's icon), the replay verdicts (用/部/無) and libraries — via `<Kanji icon mode>`.
Then the dōjō nav.

**Open questions:**
- #133: project glyphs in icon mode. Keep each project's inferred kanji (recommended), or use one generic icon?
- Run `make dev-signing-cert` once so local installs keep their keychain and folder grants. #282 (releases)
  needs an Apple Developer account.
- #199: on headless Linux, an encrypted-file credential store, or stop publishing Linux daemon artifacts?
- #194: drop the 145k/day `task_started`/`task_completed` rows from `public.logs`?
- The orphaned Cloudflare build trigger fails every develop push. Delete it in the dashboard?
- 8 CodeQL `rust/cleartext-logging` alerts in test files need triage.

**Known-broken / pending:**
- dōjō sync on this machine fails with "keychain access needs approval" until you click "Approve keychain
  access" (sign-in overlay or Settings → dōjō). Its GitHub token is also expired, so a sign-in follows.
- #284: some inspect screenshots capture a blank webview.
- `/api/projects/{id}/icon` 404s for projects whose checkout is gone. Dependabot reports 0 alerts (#172).
- The committed `database/design.dbml` is stale (pre-existing).
