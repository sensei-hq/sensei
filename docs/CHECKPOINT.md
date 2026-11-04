# Checkpoint — 2026-11-04 evening (coordinator; lanes in ~/Developer/sensei-hq/coord/TRACKER.md)

**Slice:** the week's spine: sync verified → targets end to end → a miss becomes a fix and a learning. Five lanes
(L1–L5) + Design; reviews through `~/Developer/sensei-hq/coord`.

**Merged today:** `lane/L6` docs §3.3 (doc-links allowlist now empty), `lane/L4` Notice family (Design r3),
`lane/L1` F4 viewer pools ≥2 people (also fixes develop's CI: fresh-DB `permission denied for schema sensei`).
Merged tree: full `cargo test -p senseid` 4270 passed / 0 failed / 60 ignored (own template `sensei_test_l0`);
dōjō SQL 63/64 (the one failure is noted under Known broken).

**Decided today:** D87 the screen shell (chrome only, in `@rokkit/app`; panels remembered per view in `appState`;
stats filter the screen; free-form search; migrations lose nothing); D88 past due hides private repos, payment
reinstates them. Module tree brief for L1 (kind → language → crate/package; Svelte files are components).

**Remaining, in order:**
1. L3 `run.sh --apply` → WP8 → WP9A → D18 label → #314 production sync (critical path; TG2 waits on WP8).
2. L1 #156 r2 + D83 batch A tests (review running) → module-tree analysis → diagrams onto the shell.
3. L2 actionitems-lows tests r3 → TG2–TG5 → targets UI. L4 pack-rule tests r2 → metric_readings.
4. L5 rebase onto Notice → libraries + knowledge-sources merge. Design: billing r3, screen-shell r2 + contract →
   L0 builds the rokkit shell.
5. **Install** with the user: converge the live DB twice (ActionItems, TG1 both planes incl. `staging.metric_gating`,
   F4 CHECK, SDLC tables), `make install` (Keychain), live checks, F4 re-push.

**Next command:** review/merge as lanes report; then the install checklist in coord/TRACKER.md.

**Open:** #350's allowlist vs TG1 `coalesce(modified_at, now())`; does `staging.metric_gating` need RLS (asked L2).

**Known broken:** `database/tests/dojo/target_metric_gating.sql` fails until `staging.metric_gating` is converged into
the local dōjō DB (install step). The installed daemon (28 Oct) predates S3 (e2e schema test).
