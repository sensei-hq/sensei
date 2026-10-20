# Checkpoint — 2026-10-20

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md). Waves 1 and 2 done; wave 3 in progress.

**Done:**
- Waves 1–2 done (see burndown). Wave 3: #104, #185, #186, #153, #154, #177, #157, #280, #213, #135, #179, #181, #96, #285, #286, #85, #144 closed;
  #133 implementation complete (awaiting two decisions).
- #283 live: `running_since` + "running for", a per-pass limit ("still running after Ns"), a daemon that never
  prompts for the keychain, and an "Approve keychain access" action.
- #290 closed: files counted where declarations are (Atlas 2,096 modules / 11,823 exports; rework_density back).
- #288 closed: no fallback reads remain (72 → 0, `get` deleted, guarded); every screen verified live.
- #287 closed: rework placed by path, once per event; Patterns folds it into one "rework hotspots" (618 → 2).

**Remaining:**
- #293 step 0: app done (522b8092). `Screen` moved to `packages/ui` (@sensei/ui), shared with the dōjō (a9a0a1d9); all 22 dōjō Scr screens on it (c9f1e0d7). A local-only verify user exists in local Supabase (admin role).
- Keychain: a Team-ID-less senseid is partitioned by cdhash, so each daemon reinstall revokes approval (#283 comment). UI-only work uses `make app-e2e-bundle` (no daemon reinstall).
- #298: project Instruments takes 11–28s (mcp-tool-stats pairs every tool call). `jerrythomas` connected (CLI sign-in); its sync denies all 22 repos `not_elected_user` until the user elects repos. #294/#295 filed.
- #283: the approve round trip needs a stable signing identity first (see Open questions), then close.
- Open: #292, #149, #278, dōjō (#132, #106, #191), code health (#199, #201, #139).
- #190 (wave 5): a `modules:` design cannot be imported until `project::survey` + `Workspace::load` land.
- Wave 0 leftovers: #240, #184. Wave 9: the full re-index. Wave 10: Java/C#/Python parity (#275, #276).
- #187 needs `make test-app-e2e-cold` (it stops postgres and ollama). #194 waits on a live failure.

**Next command:** `/sensei:session refocus`, then #293 step 0 for the dōjō routes: move the `/you/dojos` bar into ScrMyDojos (its column scrolls today), the inbox/run panes and invite acceptance onto Screen, then make AppShell's main column non-scrolling (+ guard). Then #293 steps 1–2. Also open: #132 step 3, #292, #106, #191, #201, #139; #278 after #289's dedupe.

**Open questions:**
- #289: delete the 167,932 duplicate assistant events (after `make db-backup`)? Recommended.
- #149: communities — Leiden/Louvain (recommended, measured the same way) or hierarchy grouping?
- #133: project glyphs in icon mode — keep each project's inferred kanji (recommended), or one generic icon?
- #133: the dōjō glyph default — kanji (recommended, shipped on develop) or icons?
- #282 (releases) needs an Apple Developer account.
- #199: on headless Linux, an encrypted-file credential store, or stop publishing Linux daemon artifacts?
- #194: drop the 145k/day `task_started`/`task_completed` rows from `public.logs`?
- The orphaned Cloudflare build trigger fails every develop push: delete it? 8 CodeQL test-file alerts to triage.

**Known-broken / pending:**
- dōjō sync still fails with "keychain access needs approval" (#283): senseid is now signed with `sensei-dev`;
  click Approve keychain access → Always Allow once. A GitHub sign-in follows.
- #284: inspect captures stay blank until `make dev-signing-cert` + a one-time Screen Recording grant.
- `/api/projects/{id}/icon` 404s for projects whose checkout is gone. `database/design.dbml` is stale.
