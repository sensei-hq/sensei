# Checkpoint — 2026-10-20

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Waves 1 and 2 done; wave 3 in progress.

**Done:**
- Wave 1: #271, #269. Wave 2 decided (#141, #230, #90 closed; #277, #278, deferred epic #279; #275).
- Wave 3: #104, #185, #186, #153, #154, #177, #157, #280, #213, #135, #179, #181, #96 closed.
- #283 live: `running_since` + "running for", a per-pass limit ("still running after Ns"), a daemon that never
  prompts for the keychain, and an "Approve keychain access" action.
- dbd 0.27.3 (2d77d771), rustls 0.23.46 (c11e19d2).

**Remaining:**
- #283: verify the approve round trip after the user clicks it (then close). #285 next, then the rest of wave 3
  (#85, #132, #106, #191, #199, #201, #139, #144, #149, #278, #133) and waves 4–10.
- #190 (wave 5): a `modules:` design cannot be imported until `project::survey` + `Workspace::load` land.
- Wave 0 leftovers: #240, #184. Wave 9: the full re-index. Wave 10: Java/C#/Python parity (#275, #276).
- #187 needs `make test-app-e2e-cold` (it stops postgres and ollama). #194 waits on a live failure.

**Next command:** `/sensei:session refocus`, then #285: router-key reads report `unreadable: <reason>`
rather than "no key" (`gateway_routers.rs`, `gateway_init.rs`).

**Open questions:**
- Run `make dev-signing-cert` once so local installs keep their keychain and folder grants. #282 (releases)
  needs an Apple Developer account.
- #194: drop the 145k/day `task_started`/`task_completed` rows from `public.logs`?
- The orphaned Cloudflare build trigger fails every develop push. Delete it in the dashboard?
- 8 CodeQL `rust/cleartext-logging` alerts in test files need triage.

**Known-broken / pending:**
- dōjō sync on this machine fails with "keychain access needs approval" until you click "Approve keychain
  access" (sign-in overlay or Settings → dōjō). Its GitHub token is also expired, so a sign-in follows.
- #284: some inspect screenshots capture a blank webview.
- `/api/projects/{id}/icon` 404s for projects whose checkout is gone. Dependabot reports 0 alerts (#172).
- The committed `database/design.dbml` is stale (pre-existing).
