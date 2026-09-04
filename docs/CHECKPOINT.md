# Checkpoint

**Slice:** capability-trait refactor — `docs/spec/2026-09-02-capability-refactor-map.md`
(+ the ADR beside it). Branch `develop`.

## Done

- **Slice 0** — vendored exclusions (`0a31582e`). Nodes 728,742 → 399,722 (−45.1%);
  0 nodes under the excluded path. The bug was the SILENCE: an exclusion entry that
  resolved to a path matching nothing, with nothing saying so.
- **Slice 1** — one adapter registry + a capability matrix that can't lie (`e49b1ac1`).
- **Slice 1b** — FQN for kotlin, C, swift (`030e34db`). `KNOWN_GAPS` retired rather
  than zeroed; `every_adapter_supports_fqn_with_no_exceptions` gates it.
  `fqn_output` now takes `rel_path` (folder-relative, = `nodes.file_path`).
- App unit tests no longer need a live daemon (`335d2421`).
- **Graph layout kinds** named + DDL-pinned; dead `app/tests/graph.test.ts` removed
  (`21b0cd85`). That file was unreachable by any runner and asserted `e.type` where
  the API emits `kind` — apparent coverage for the endpoint slice 2 must not break.
- **Slice 2, increments 1–4 DONE:**
  - `49725e7b` `RelationKind` {Extends, Implements, TraitImpl} + the
    `edges.props.relation` discriminant, DDL-guarded. ADR amended: its `Mixin`
    variant dropped because `IRClass.mixins` has no writer.
  - `e6522e20` `TypeRelation` on `FqnFileOutput` + the rust trait-impl producer.
    MEASURED over the real tree: 112 relations / 386 files, 107 (95.5%) resolving.
    Extracted `PathClass::to_fqn` as the one owner (would have been a third copy).
  - `97f4f44a` `insert_edge_with_props`. Before it, 0 of 724,926 edge rows had
    props because no code path named the column. Merges, never clobbers.
  - `5364d862` `implements` added to `GRAPH_LAYOUT_KINDS` + `COMMUNITY_EDGE_KINDS`,
    both hoisted to pinnable constants first.

## LIVE IN THE GRAPH (deployed 13:07, verified)

| | before | after |
|---|---|---|
| kotlin fqn | 0 / 1,542 | **1,709 / 2,062 (82.9%)** |
| c fqn | 1 / 322 | **325 / 442** |
| swift fqn | 0 / 8 | **8 / 9** |
| inheritance edges | 0 | **2,677 — 2,672 resolved (99.8%)** |
| by language | — | java 2,169 · python 386 · rust 122 |
| by relation | — | extends 1,553 · implements 1,002 · trait_impl 122 |
| syntax leakage in supertype names | — | **0** |
| mislabelled `extends` | 7,916 (0 usable) | **0 — swept at reconcile** |

GRAPH VIEW BEFORE/AFTER (the comparison that was an explicit decision): the
Atlas previously received 7,916 `extends` rows of which **0 were usable**
(every one had a NULL target). It now receives 0 of those and 112 `implements`
rows, **107 usable**. The layout traded 7,916 dead rows for 107 real edges.

Deploying found a defect no test would have: `Labs/Bezier3D` came back with only
`file`/`module` nodes because `adapter_for_filename` derived the extension from
the ORIGINAL name while adapters declare theirs lowercase — every
uppercase-extension file was silently skipped. Fixed in `9c6c72eb`; that folder
now yields 19/18 nodes.

TWO CORRECTIONS TO MY OWN CLAIMS about the C header/impl collision fix:
1. Those files were never parsed at all (uppercase extensions), so no collision
   was occurring — I inferred a live bug from path shapes without checking that
   the files produced symbols.
2. Production splits `Bezier3D/Cpp` and `Bezier3D/Hpp` into SEPARATE folders, so
   `rel_path` carries no directory prefix and both collapse to `c·ADVMATH`
   anyway. Harmless — `node_id_by_fqn` is folder-scoped — but it means the fix's
   scenario (both trees in ONE folder) is still unexercised by this corpus. The
   unit test passes; production does not test it.

## Remaining

Slices 2–10 in the map: `Inheritance` persistence → `GraphFacts` + persister →
`OnMiss` migration → `ImportPaths` → `LibraryOrigin` → externals as `lib_symbol`
(136,642 edges) → `Components` → `FolderRow` → `NodeRow`/`EdgeRow` on touch.

## Next command

SLICE 2 COMPLETE (11/11). SLICE 3 COMPLETE at the approved scope 0-4:
`47a04a51` arm instrumentation, `6a8f2098` GraphFacts + shared lib-package
derivation, `dccec36c` persist_edge_fact, `c79aaa7c` inheritance migrated,
`c156239a` calls migrated. Increments 5-6 CUT as churn, with approval.

The call and inheritance arms ran the SAME three-branch ladder written twice;
they are now the same code. That was the only real duplication across the six
emit paths. Slice 3 changed no graph output BY DESIGN — its value is that a
dropped in-file fast path is now caught, which was invisible to every
pre-existing test (demonstrated by probe).

DEPLOYED 2026-09-04 10:47 and verified. Slice 2 increment 10 and all of slice 3
are now live.

MY FIRST VERIFICATION DESIGN WAS INVALID and is recorded so it is not repeated:
I compared edge counts before/after reindexing
`/Users/Jerry/Developer/sensei-hq/sensei` — the one repo whose SOURCE I had
been editing all session (deleted `extract_trait_from_impl`, removed two
ladders, ~130 lines of emit code). Fewer call edges is the expected consequence
of deleting code, not evidence about the refactor. The confound is total, and
the old binary was gone so no clean baseline could be retaken.

WHAT DID VERIFY IT — invariants that need no before-snapshot, all holding with
the persister live:
- props asymmetry EXACT: calls/imports/references 0% stamped,
  extends/implements 100%. A persister that stamped uniformly would break this.
- `target_id` xor `target_name`: 0 rows have both, so resolving still erases
  the name.
- inheritance resolution intact: extends 1,553/1,553, implements 1,253/1,260.
- both stub kinds live and distinct: 45,479 `function` stubs vs 254 `class`
  stubs, proving the per-caller `kind` on the policy still fires.

THEN: slice 4 — `ImportPaths` trait, then `LibraryOrigin` and externals as
`lib_symbol` (136,569 import edges, only 25,788 resolved — the largest
remaining resolution gap in the graph).

## Open questions

None blocking. Two resolved by fiat and documented in code/ADR: props key is
`relation`, and every inheritance edge is stamped uniformly (so "implements X
excluding rust trait impls" stays a pure props predicate).

## Known broken

- Increment 10 (rust IR trait fix) is committed but NOT redeployed, so its
  effect is unverified in the graph. Needs install + reindex.
- `nodes.language` is last-writer-wins on the reference path (`graph.rs:392`
  lacks the `CASE WHEN EXCLUDED.resolved` guard its neighbours have), so a
  cross-language reference can relabel a node. Pre-existing.

- Slice 1b is **not live in the graph**: the daemon runs the installed 0.9.1
  binary. Needs `make install-debug` + reindex, batched with later slices.
- 44,689/46,117 unknown stubs unresolved (java-dominated).
- TS/JS locally-declared-names pre-pass missing (`t` 573, `fn` 189, `setLoading` 235).
- `libraries.rs` holds a third import-classifier copy (inert — writes a dead field).
- 7,418 `nodes.file_path` rows are absolute.
- #149 community re-measure; #150 content-hash identity (684 multi-folder groups).

## Graph state

399,722 nodes / 46,516 files / 8,855 folders. Edge resolution: calls 65.1% ·
references 30.8% · imports 18.9% (every LOCAL one) · extends 0% — and `extends`
is misnamed containment, not inheritance.

## Traps

Run the full gate with the daemon STOPPED (`metrics_pipeline_end_to_end` flakes
under contention) — but the APP suite needed a daemon until `335d2421`, so run
both. Never `git checkout --` to undo a mutation; use a `cp` backup. Never pipe a
gate through `| head` (SIGPIPE masked a real `fmt` failure). The leak hook rejects
real home paths — use `/Users/dev/`. `sensei_test` has no schema provisioning, so
DDL changes need applying to both DBs. Verifying an indexer fix needs
`delete from sensei.scan_state` first.

## Lessons worth keeping

- **Real data refutes fallbacks that reasoning approved.** C's no-build-root
  fallback used the bare file stem; a corpus project with parallel `Cpp/`/`Hpp/`
  trees made a header and its implementation share an fqn. My fixtures had agreed
  with the bug.
- **A green corpus test doesn't mean the branch you changed is covered.** Mutating
  that fallback did not fail the corpus test, because this repo's root `Makefile`
  routes every `.c` file through the build-root branch instead. Probe the mutation;
  don't infer coverage from a pass.
- **Assert every scripted edit.** Two `python3` replacements silently no-op'd
  because rustfmt had already rewrapped the target lines.
