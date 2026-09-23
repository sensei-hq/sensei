# Checkpoint

**Slice:** indexer v2 — flipping TypeScript into `PRODUCTION_LANGUAGES` (issue #130, phase `build`)

## Done

- **Observability views** (`c00f7406`) — `graph_nodes`/`graph_resolution` carry `repository`; `activity.task_health` + `task_failures` are the restart list; `sensei.error_signature` groups failures by cause. Failure lake reclaimed: 17,577,049 rows, 14 GB → 656 MB.
- **Stale reachability comments** (`900ec06c`, `daf428fb`) — `reconcile.rs` "no caller on purpose" and `indexer/mod.rs` "deliberately without a caller" both outlived the rust cutover. Dropping reconcile's blanket `allow(dead_code)` exposed exactly one real gap: `Stated::Gone` (the deletion path) is never constructed.
- **TypeScript A7: 511 → 10** (`daf428fb`) — one rule, four shapes. A function body names what it declares:

  | fix | after |
  |---|---:|
  | baseline | 511 |
  | call-argument callbacks | 38 |
  | class method bodies (`container_at`, ported from rust) | 14 |
  | functions under an object key | **10** |

## Remaining

1. **Decide the A7 endgame for TS.** The 10 are one shape: two `const` in different *block* scopes of one function, plus one type declared twice. Either add block-level naming to reach 0, or ratchet TS at 10 with the shape documented. **The ratchet test is rust-scoped** (`no_two_declarations_in_this_repos_rust_mint_one_identity`) — a TS-scoped one must be added or the flip has no gate.
2. **Flip** — add `Language::TypeScript` to `PRODUCTION_LANGUAGES` (`lang/mod.rs:426`) + delete `languages/typescript.rs`, `languages/javascript.rs`.
3. **Deploy, clear, re-index, acceptance.**
4. **Wire `Stated::Gone`.**
5. Port SQL, Swift, Kotlin, Vue, C; then delete `languages/`.

## Next command

```
cargo test -p senseid --bin senseid indexer::acceptance -- --ignored --nocapture
```
(do **not** pipe it — a pipe reports the pipe's exit status, not the test's)

## Open questions

- Block-level naming for TS locals: reaches A7 zero, at the cost of fqn churn on edits within a function. Rust reads zero on the same rule only because its corpus doesn't hit it.
- Flipping `TypeScript` flips `.ts/.tsx/.cts/.mts/.js/.jsx/.mjs/.cjs` **and** `.svelte` together — one `Language`, one fqn scheme. The spec's "js, ts, svelte one at a time" is not expressible. `.vue` has no v2 adapter, so it stays on v1.

## Known-broken / not deployed

- Daemon binary predates the v2 wiring **and** the `mark_file_parsed` fix (`parsed_at` is 0 on all 111,276 files).
- **Clearing before re-index is required, not optional**: reconcile reads prior claims via `props->'claims'`, a v2-only marker that **0 of 467,707** existing nodes carry — v1 rows are invisible to it and would never be demoted.
- `graph_resolution.resolved_via` is NULL on all 965,518 edges: the only `rung` writer is v2's `persist.rs`, and every production edge came from v1.
- The **differential harness is not being built** — superseded. `docs/design/indexer.md:757-759` opens §6 with "Thresholds, not comparisons"; `13-cutover.md` is `status: superseded`.
