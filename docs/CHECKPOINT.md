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

## Remaining

Slices 2–10 in the map: `Inheritance` persistence → `GraphFacts` + persister →
`OnMiss` migration → `ImportPaths` → `LibraryOrigin` → externals as `lib_symbol`
(136,642 edges) → `Components` → `FolderRow` → `NodeRow`/`EdgeRow` on touch.

## Next command

Slice 2 (`Inheritance`). java/python/rust already EXTRACT inheritance and nothing
reads it, so this slice is persistence, not parsing. Measure the graph view at
`codebase.rs:55` BEFORE retiring the 7,901 mislabelled `extends` containment
edges — that before/after was an explicit decision.

## Open questions

None blocking.

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
