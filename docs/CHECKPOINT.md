# Checkpoint

**Slice:** parser import collection. Last commit `95ed30f1`. Branch `develop`.

The adversarial review found my §12 root cause FALSE (all four producers already
build import registries) and named four concrete bugs it missed. **All four are
fixed and pushed** — they needed none of the design work.

## Done this run

`95ed30f1` — one grammar-driven binding reader per language, shared by the FQN map
and both import records. Five string-splitting copies removed; three duplicated
helpers (`collect_use_tree`, `join_mod`, `parent_mod`) deleted from `mod rust_fqn`.

| defect | scale in this repo |
|---|---|
| nested + multi-line group `use` mis-parsed → `{Path`, `State}` | 95 multi-line, 28 nested |
| only the first `super::` consumed | 52 statements |
| function-local `use` never collected (file-global now wins a collision) | 740 statements |
| Python aliased plain imports produced **zero** records | all `import x as y` |
| fell out: `pub use` carried keywords into the path; `as` left `Error as IoError` | 60 re-exports |

`languages/corpus_tests.rs` — negative invariants over the working tree's 1,398
source files. Pre-fix **66** mangled names / **65** keyword paths / **111** FQNs
retaining a navigation segment; post-fix 0 / 0 / 0.

- **Gates at `95ed30f1`:** 2656 senseid, 174 bootstrap, 1698 app unit / 118 files,
  clippy `-D warnings` 0, fmt clean.
- **Mutation-probed:** reverting the `entry()/or_insert` precedence rule fails the
  collision test; restoring HEAD's two parsers fails all three corpus invariants.

## Next command

Nothing is in flight. Next slice is the identity-fabrication branches —
`rust_lang.rs:981`, `rust_lang.rs:1171`, `typescript.rs:1018`, `java.rs:620` —
which must land **with** stub GC: `delete_edges_from_sources` removes the in-edges
that prove the fix worked, and `prune_file_nodes` filters `file_path = $2` while
stubs have `file_path IS NULL`, so 84,379 stub nodes have no GC path.

Invariant is **stub count → 0**, not resolution rate: a correct fix makes
resolved% FALL (18% → ~31% of a smaller denominator).

## Open questions

- **`folders.props.libs` derives from `target_id IS NULL`** (`resolve.rs:92-102`),
  so resolving import edges makes the Observatory report zero dependencies. Needs
  a decision BEFORE the next slice.
- `extends` has two scheduled fates in the sketch while `codebase.rs:55` already
  consumes it.
- **#138 project/namespace** is a decision, not a task — the repository rung does
  not exist, so project aggregation would aggregate an empty set.
- **#137** blocks 4,329 user-scope sync rows (no writer sets
  `personas.principal_id`).

## Known-broken

- `docs/analysis/graph-end-state-sketch.md` §1–12 remain **NOT SAFE TO BUILD
  FROM**; the correction block is authoritative.
- **`references` targets are not resolvable data** — `/` (2,000 edges), `id`,
  `true`, `429`. `doc_indexer.rs:587` joins absolute paths, discarding the repo
  root (54,740 absolute targets). `references` does NOT unlock `rationale_for`:
  0 of 1,700 rationale nodes have any out-edge.
- **`dojo_memberships.sync_status` is dead** — `memberships::set_sync_status` has
  zero callers, so the connections pane's "healthy" count is permanently 0.
- **Removing an exclusion enqueues a re-scan that races the prune** — harmless now
  that the scan honours exclusions, but the sequencing is unguarded.
- `/settings/projects` e2e fails on a fresh DB (#134). e2e populated-path gap on
  settings-metrics (needs a `sensei.repositories` row).
- **The security-reminder hook false-positives** on the regex `match` sibling
  method in ANY file including Markdown.

## Filed, not started

#131–#136, #139–#142, #144, #145, #146.
