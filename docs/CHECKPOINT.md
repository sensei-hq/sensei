# Checkpoint

**Slice:** indexer v2 — TypeScript cut over; next is deploy + wipe + re-index (issue #130, phase `build`)

## Done

- **Observability views** (`c00f7406`) — `repository` on the graph views; `activity.task_health` + `task_failures` as the restart list. Failure lake reclaimed: 17,577,049 rows, 14 GB → 656 MB.
- **TypeScript A7 ratchet at ZERO** (`daf428fb`, `0efc151f`) — 511 → 0. The last step was not naming things harder but declaring fewer: **a local value is not a node**. Barrier numbers unchanged to the digit, so nothing reader-facing was lost.
- **TypeScript CUT OVER** (`2e18e352`) — `PRODUCTION_LANGUAGES = [Rust, TypeScript]`. `languages/typescript.rs`, `svelte.rs`, `vue.rs` deleted (2,426 lines; net **-2,416**). Vue was **migrated** to `indexer/lang/vue.rs`, not grandfathered.

## Remaining

1. **Deploy, wipe, re-index** ← NEXT
2. **Wire `Stated::Gone`** — the deletion path has no production constructor.
3. Port SQL, Swift, Kotlin, C; then delete `languages/` and break its two couplings.

## Next command

```
make install-service && \
psql -h localhost -p 5432 -d sensei -c 'TRUNCATE sensei.nodes, sensei.edges CASCADE;'
```
Then a full re-index, then acceptance against the live graph.

Do **not** pipe test/build commands through `tail` — a pipe reports the pipe's exit status.

## Open questions

- Re-embedding is the real cost of the wipe (~326,716 nodes last measured). Record throughput before anything depends on it.
- `.ts`, `.js` and `.svelte`/`.vue` share one `Language`, so they flipped together. The spec's "one at a time" is not expressible through a per-language frontier.

## Known-broken / not deployed

- **The daemon predates all of it** — the v2 wiring, the `mark_file_parsed` fix (`parsed_at` is 0 on all 111,276 files), and this cutover.
- **The wipe is required, not optional**: reconcile reads prior claims via `props->'claims'`, a v2-only marker that **0 of 467,707** nodes carry. v1's rows are invisible to it and would never be demoted.
- `graph_resolution.resolved_via` stays NULL until the re-index — the only `rung` writer is v2's `persist.rs`.
- Failure retention still has no cap on repeated identical failures, so the lake can refill.
