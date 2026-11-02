# Checkpoint — 2026-11-02 (diagrams as facets; load-time fixes; Muse on dōjō sync)

**Slice:** the user's 2026-11-02 review — plan in `docs/plan/diagrams-facets.md` (S0–S5).

**Done (develop, pushed):**
- S0 rokkit 1.12.3 (first release from sensei-hq, trusted publishing) in app, dōjō, website.
- S1 45da00255 — zoom per diagram; What to act on is each lens's own, selectable, Copy as markdown.
- S2 2bccb831b Structure (skips amber), Matrix (issue cells + counted legend), Zones (pain &
  uselessness focus, labelled); eccf945a7 Cycles (a ring per cycle, edge to cut);
  408b58f53 Hidden coupling (ranked pairs).
- S4 9582b5578 hotspots 77 s → 54 ms (partial index); 29240b694 metrics daily 42k → 1.2k
  buffers (`repository_workspace` view); live DB converged with dbd reconcile (twice).
- S5 b6deb16d0 `docs/plan/dojo-sync-handoff.md` (7 packages, 6 open questions).

**Remaining, in order:**
1. S2 Complexity as a tree: daemon read aggregating `sensei.file_complexity` module → folder →
   file + maintainability grade; `ComplexityLens` tree.
2. S4 #3 narration_cache batched update, #4 dated series in `/metrics` (drop 27-request fan-out),
   #5 drill-down parallel load, #6 `GET /api/workspaces/{id}`; overview not blocking on hotspots.
3. S3 Database · ER = dbd viewer: port Overview/Object/Enum views + sidebar into
   `@rokkit/graph/schema`, rokkit release, compose the sensei page.
4. Live check of all diagram lenses in the installed app (quit Sensei.app before inspect).

**Next command:** `cargo test -p senseid --bin senseid complexity` after writing the tree read's test.

**Open:** S4 2c — ratio pooling sums per-commit rows of one repo/day (possible double count).
Muse (pid 24357) is editing dōjō sync in THIS checkout (dojo_sync.rs, sync.rs, user_plane.rs):
the pre-commit hook runs over the whole tree, so its WIP can block commits; a worktree for it
would isolate both. dōjō pulls a second rokkit (1.4.0) via `@kavach/ui` 1.3.0's exact
`@rokkit/forms@1.3.7` pin — fix in kavach.

**Known broken:** Keychain test on rebuilt binaries (#282). Host load ~100 (a VM at ~1300% CPU).
