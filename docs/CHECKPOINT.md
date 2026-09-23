# Checkpoint

**Slice:** indexer v2 — flipping TypeScript into `PRODUCTION_LANGUAGES` (issue #130, phase `build`)

## Done

- **Observability views** (`c00f7406`) — `graph_nodes`/`graph_resolution` carry `repository`; `activity.task_health` + `task_failures` are the restart list; `sensei.error_signature` groups failures by cause. Failure lake reclaimed: 17,577,049 rows, 14 GB → 656 MB.
- **Stale reachability comments** (`900ec06c`, `daf428fb`) — `reconcile.rs` "no caller on purpose" and `indexer/mod.rs` "deliberately without a caller" both outlived the rust cutover. Dropping reconcile's blanket `allow(dead_code)` exposed exactly one real gap: `Stated::Gone` (the deletion path) is never constructed.
- **TypeScript A7 is at ZERO** (`daf428fb`, `0efc151f`):

  | step | after |
  |---|---:|
  | baseline | 511 |
  | call-argument callbacks | 38 |
  | class method bodies (`container_at`, from rust) | 14 |
  | functions under an object key | 10 |
  | **a local value is not a node** | 1 |
  | `module_of` reads `module_here` (types scoped like items) | **0** |

  Dropping local values removed 5,074 declarations and the ts barrier numbers were **unchanged to the digit** (2,186 / 1,454 / 935 / 767 / 422) — the proof none of it was reader-facing. A3 moved exactly 222 resolved→unresolved, conserved.
- **`no_two_declarations_in_this_repos_typescript_mint_one_identity`** added — it never existed, which is why 511 could accumulate. Mutation-verified red.

## Remaining

1. **Flip** — add `Language::TypeScript` to `PRODUCTION_LANGUAGES` (`lang/mod.rs:426`) + delete `languages/typescript.rs`, `languages/javascript.rs`.
2. **Deploy, clear, re-index, acceptance.**
3. **Wire `Stated::Gone`.**
4. Port SQL, Swift, Kotlin, Vue, C; then delete `languages/`.

## Next command

```
cargo test -p senseid --bin senseid indexer::acceptance -- --ignored --nocapture
```
(do **not** pipe it — a pipe reports the pipe's exit status, not the test's)

## Open questions

- Flipping `TypeScript` flips `.ts/.tsx/.cts/.mts/.js/.jsx/.mjs/.cjs` **and** `.svelte` together — one `Language`, one fqn scheme. The spec's "js, ts, svelte one at a time" is not expressible. `.vue` has no v2 adapter, so it stays on v1.

## Known-broken / not deployed

- Daemon binary predates the v2 wiring **and** the `mark_file_parsed` fix (`parsed_at` is 0 on all 111,276 files).
- **Clearing before re-index is required, not optional**: reconcile reads prior claims via `props->'claims'`, a v2-only marker that **0 of 467,707** existing nodes carry — v1 rows are invisible to it and would never be demoted.
- `graph_resolution.resolved_via` is NULL on all 965,518 edges: the only `rung` writer is v2's `persist.rs`, and every production edge came from v1.
- The **differential harness is not being built** — superseded. `docs/design/indexer.md:757-759` opens §6 with "Thresholds, not comparisons"; `13-cutover.md` is `status: superseded`.
