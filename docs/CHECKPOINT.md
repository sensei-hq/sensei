# Checkpoint — 2026-10-20

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Waves 1 and 2 done; wave 3 in progress.

**Done:**
- Waves 1–2 done (see burndown). Wave 3: #104, #185, #186, #153, #154, #177, #157, #280, #213, #135, #179, #181, #96, #285, #286, #85, #144 closed;
  #133 implementation complete (awaiting two decisions).
- #283 live: `running_since` + "running for", a per-pass limit ("still running after Ns"), a daemon that never
  prompts for the keychain, and an "Approve keychain access" action.
- #287 scope live (fcb71823, 2228a97a): rework placed by path in the project's indexed repos, once per event;
  617 → 371 rows here, none outside. Backfill resets by project_id (12 projects had reset nothing).

**Remaining:**
- #283: verify the approve round trip after the user clicks it (then close). #133 slice 1 done; then #286,
  #286, #85, #144, #149, #278, dōjō (#132, #106, #191), code health (#199, #201, #139).
- #190 (wave 5): a `modules:` design cannot be imported until `project::survey` + `Workspace::load` land.
- Wave 0 leftovers: #240, #184. Wave 9: the full re-index. Wave 10: Java/C#/Python parity (#275, #276).
- #187 needs `make test-app-e2e-cold` (it stops postgres and ollama). #194 waits on a live failure.

**Next command:** `/sensei:session refocus`, then #288 (in progress, 40 fallback reads left: Impact/Traceability next), #290 (no 'file' nodes),
dōjō (#132, #106, #191), code health (#201, #139). #278 closes after #289's dedupe + a re-measure.

**Open questions:**
- #287 granularity: serve rework as one "rework hotspots" pattern grouped by directory (recommended), keeping
  per-file rows as the signal behind `rework_density` and the per-file recommendations?
- #289: delete the 167,932 duplicate assistant events (after `make db-backup`)? Recommended.
- #149: communities — Leiden/Louvain (recommended, measured the same way) or hierarchy grouping?
- #133: project glyphs in icon mode — keep each project's inferred kanji (recommended), or one generic icon?
- #133: the dōjō glyph default — kanji (recommended, shipped on develop) or icons?
- Run `make dev-signing-cert` once so local installs keep their keychain and folder grants. #282 (releases)
  needs an Apple Developer account.
- #199: on headless Linux, an encrypted-file credential store, or stop publishing Linux daemon artifacts?
- #194: drop the 145k/day `task_started`/`task_completed` rows from `public.logs`?
- The orphaned Cloudflare build trigger fails every develop push: delete it? 8 CodeQL test-file alerts to triage.

**Known-broken / pending:**
- dōjō sync on this machine fails with "keychain access needs approval" until you click "Approve keychain
  access" (sign-in overlay or Settings → dōjō). Its GitHub token is also expired, so a sign-in follows.
- #284: inspect captures stay blank until `make dev-signing-cert` + a one-time Screen Recording grant.
- `/api/projects/{id}/icon` 404s for projects whose checkout is gone. `database/design.dbml` is stale.
