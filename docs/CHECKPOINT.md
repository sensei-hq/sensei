# Checkpoint — 2026-10-20

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Waves 1 and 2 done; wave 3 in progress.

**Done:**
- Waves 1–2 done (see burndown). Wave 3: #104, #185, #186, #153, #154, #177, #157, #280, #213, #135, #179, #181, #96, #285, #286, #85, #144 closed;
  #133 implementation complete (awaiting two decisions).
- #283 live: `running_since` + "running for", a per-pass limit ("still running after Ns"), a daemon that never
  prompts for the keychain, and an "Approve keychain access" action.
- #288 closed: no fallback reads remain (72 → 0, `get` deleted, guarded); every screen verified live.
- #287 closed: rework placed by path, once per event; Patterns folds it into one "rework hotspots" (618 → 2).

**Remaining:**
- #283: the approve round trip needs a stable signing identity first (see Open questions), then close.
- Open: #290, #291, #149, #278, dōjō (#132, #106, #191), code health (#199, #201, #139).
- #190 (wave 5): a `modules:` design cannot be imported until `project::survey` + `Workspace::load` land.
- Wave 0 leftovers: #240, #184. Wave 9: the full re-index. Wave 10: Java/C#/Python parity (#275, #276).
- #187 needs `make test-app-e2e-cold` (it stops postgres and ollama). #194 waits on a live failure.

**Next command:** `/sensei:session refocus`, then #290 (no 'file'/'doc' graph nodes: Atlas modules, rework_density,
doc drift), #291, dōjō (#132, #106, #191), code health (#201, #139). #278 closes after #289's dedupe + a re-measure.

**Open questions:**
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
- dōjō sync still fails with "keychain access needs approval": senseid is signed ad hoc, so each rebuild drops
  the grant (#283). `make dev-signing-cert` → reinstall → Approve, Always Allow. A GitHub sign-in follows.
- #284: inspect captures stay blank until `make dev-signing-cert` + a one-time Screen Recording grant.
- `/api/projects/{id}/icon` 404s for projects whose checkout is gone. `database/design.dbml` is stale.
