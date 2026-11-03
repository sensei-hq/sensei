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
- Metrics page state 17771862a (rule a3b74a4bd: load() default, page state when slow): draws at
  once, fills by section, sparklines 4 at a time, reads cached across landing and details.
- Complexity tree 63bf5a291 (daemon) + bb1848f40 (lens: rokkit Tree, overall reading, grade).
- Load time: 2cd0fcbb2 one-workspace read; 41b22eb3d narration cache in one statement;
  19ced3153 dated series in /metrics (no sparkline reads). App + daemon ship together.
- D46 (personal dōjō free incl. private; org private paid, hidden until subscribed) 1d51b78a5;
  WP8 in the handoff plan. WP1 (Muse) gate: one defect left (duplicate key within unmapped).
- S3 Database · ER = dbd's viewer: rokkit 1.13.0 (port a76190b7b…, released), enums in the payload
  bd9e71d27, page 4e26da8c2 (sidebar right; Overview/Diagram/Entities; table/object/enum pages).
- WP1 86b84ea46 + WP2 4370f28fb (Muse, reviewed); ratchet 1,060 → 1,075 faa167d8e.
- S5 b6deb16d0 `docs/plan/dojo-sync-handoff.md` (7 packages, 6 open questions).

**Remaining, in order:**
1. Live check — `make install` (daemon + app together; re-approve the daemon's Keychain), one sync
   pass, then: Settings › Dōjō repository rows (≈157, remedies), WP1 acceptance query, every lens,
   Database · ER (`bun run test:inspect` schema-inspect), Metrics and workspace-window timings.
2. S6 (proposal, awaiting the user): diagram snapshots per commit, animate over commits/days/sprints.
3. Muse: WP3 in progress (tracked graph branches); WP8 (D46) queued.

**Next command:** `make install` (with the user present for the Keychain approval).

**Open:** S4 2c — ratio pooling sums per-commit rows of one repo/day (possible double count).
Muse (pid 24357) is editing dōjō sync in THIS checkout (dojo_sync.rs, sync.rs, user_plane.rs):
the pre-commit hook runs over the whole tree, so its WIP can block commits; a worktree for it
would isolate both. dōjō pulls a second rokkit (1.4.0) via `@kavach/ui` 1.3.0's exact
`@rokkit/forms@1.3.7` pin — fix in kavach.

**Known broken:** Keychain test on rebuilt binaries (#282). Host load ~100 (a VM at ~1300% CPU).
---
**Also in flight (Muse · dōjō sync WP1–WP2, unpushed 86b84ea46 + 4370f28fb):**
WP1 verdict rows + WP2 Repositories section/remedies, both review-approved;
live checks with the user. Next: WP3 tracked graph branches on review go-ahead.
