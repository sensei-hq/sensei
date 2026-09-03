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
| inheritance edges | 0 | **120 trait_impl, 115 resolved (95.8%)** |

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

Slice 2 increment 6 — retire the mislabelled file-sourced `extends` emit
(`process.rs`, the parent_refs loop). Increment 5 is DONE (`722cd43e`). Then (retire the
mislabelled emit), 7 (sweep the stale rows), 8 (java), 9 (python), 10 (delete
the fabricating rust IR trait hack), 11 (`library_usage` kind filter).

Full plan + adversarial verdicts: `/private/tmp/claude-502/.../tasks/w1b00fnoq.output`.

THE THREE FIXES THAT MUST SURVIVE (adversarial review killed the naive design):
1. NO bare-name fallback. `sole_definition_id_by_name` is kind- AND
   language-agnostic (written for doc mentions), so an inheritance target would
   resolve confidently WRONG: llm-gateway defines its own `BaseModel`, so 12
   classes would point at it instead of pydantic's.
2. STUB the internal branch on a miss, mirroring the import emit. Measured:
   404/406 (99.5%) java relations have their parent in a DIFFERENT file, so
   probe-only resolution is order-dependent and never heals.
3. External supertypes need `upsert_lib_node_by_fqn`, or "which models extend
   pydantic.BaseModel" — the headline question — is what this cannot answer.

Increment 6 is now known SAFE for communities: `build_adjacency` does
`None => continue`, and all 7,905 mislabelled `extends` edges are unresolved,
so retiring them cannot move a community.

## Open questions

None blocking. Two resolved by fiat and documented in code/ADR: props key is
`relation`, and every inheritance edge is stamped uniformly (so "implements X
excluding rust trait impls" stays a pure props predicate).

## Known broken

- Slice 1b is **not live in the graph**: the daemon runs the installed 0.9.1
  binary. Needs `make install-debug` + reindex, batched with later slices.
- 44,689/46,117 unknown stubs unresolved (java-dominated).
- TS/JS locally-declared-names pre-pass missing (`t` 573, `fn` 189, `setLoading` 235).
- `library_usage.unresolved_import_count` has no kind filter (latent).
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
