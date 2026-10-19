# Checkpoint — 2026-10-19

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Waves 1 and 2 are done; wave 3 is next.

**Done:**
- **Wave 1, Settings → Projects:**
  - #271 and #269 are closed with live evidence.
  - Commits: 43b7cb6e, 650ae365, 7f533892, 1078a879, e777d692, ffc79fc6, 030592ee.
  - The live inspect (`app/e2e/tests-inspect/settings-projects-inspect.spec.ts`) passes: 3 placed,
    130 collapsed, sensei derives library/tool/website.
- **Found while verifying:**
  - `PUT /api/folders {role:null}` was a no-op 200 (e777d692).
  - The watcher rescan demoted workspace modules to `folder` (ffc79fc6). Re-verified live after
    redeploy: a watcher rescan kept all 18.
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

**Next command:** `/sensei:session refocus`, then wave 3. #104 is closed (f9ebca17..ee5e64de); next are #185 and #186 (local dōjō setup).

**Open questions:**
- The orphaned Cloudflare build trigger fails every develop push. Delete it in the dashboard?
- 8 CodeQL `rust/cleartext-logging` alerts in test files need triage.

**Known-broken / pending:**
- `/api/projects/{id}/icon` 404s for projects whose checkout is gone. The glyph falls back to kanji.
- Dependabot reports 0 alerts although the lockfiles are vulnerable (#172).
- The committed `database/design.dbml` is stale (pre-existing).
