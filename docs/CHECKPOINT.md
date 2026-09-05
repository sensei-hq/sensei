# Checkpoint

**Slice:** per-language indexer gap remediation. Branch `develop`.
Plan: `docs/spec/2026-09-02-capability-refactor-map.md` + the per-language issue
list (workflow `wwos7f4h1`, 7 agents, output under `/private/tmp/claude-502/.../tasks/`).

## Done and DEPLOYED (verified in the live graph)

- **Slices 2 + 3** complete. Inheritance 0 -> 2,677 edges at 99.8%; the call and
  inheritance ladders collapsed into one `persist_edge_fact`.
- **Slice 4** — imports **18.9% -> 100.0%** (136,562/136,573), verified live and
  survived the stub prune. Externals mint `lib_symbol` keyed on the package.
  On one untouched java folder: 0% -> 100%, of which **33% resolved to REAL
  local types** because the dotted-candidate probe fires before any mint.
  Prerequisites all landed first: repaired instrument, community guard
  (`node:test` has 3,365 importers and did NOT merge communities — 38,978
  distinct), lib-node collector (mint is reversible), genuine probe.

## Done, NOT deployed (counts are ELIGIBLE, not observed)

- `78828f7d` **issue #1** — TS/JS member call on a global receiver resolves to
  its runtime lib. `global_runtime` existed and the StaticMemberExpression arm
  never called it. ~12,371 eligible edges, ceiling 20,921.
- `2aaf6a09` **issue #2** — java third-party package becomes a lib node.
  `is_external_pkg` was a 7-prefix JDK allowlist, so every Maven package was
  minted FIRST-PARTY: org.mockito 8,528, org.junit 5,220, 17,209 total.

## Next command

Issue #3: TS/JS bare-identifier fabrication at `typescript.rs:1152-1156` —
24,621 edges, 21,106 of 21,981 function stubs (96%) are phantoms. Then #4
(kotlin emits ONLY imports: 0 calls/extends/implements against 11,493 call
sites) and #5 (`walk_stmt` is a flat single-level match, so 4,703 test files
yield 6.8% of call edges against 70,059 sites).

THEN one reindex measures #1, #2 and #3 together — they share an emit path.

## THE REAL HEADLINE, from the per-language analysis

Real definition-to-definition call linkage is **73,792 / 340,229 = 21.7%**, not
the 65.2% the raw rate shows. 93,086 calls point at FABRICATED internal stubs.
The defect is invention, not absence.

## Two of my own claims REFUTED — do not repeat them

1. **"FQNs are folder-relative and that causes the 40,651 duplicates."** FALSE
   for both major languages. Rust fqns are Cargo-package-relative; java's are
   package-qualified and repo-independent. 5,588 of 5,592 rust duplicates are
   ONE repo registered at two filesystem paths; all 22,026 java duplicates are
   `cluster` vendoring byte-identical copies of `server` and `scheduler`, all
   three registered as separate roots. It is a FOLDER-REGISTRY defect
   (duplicate repos on disk), not an fqn-scheme defect.
2. **"Tests passing means the gate held."** My gate summary printed
   `failed=0` while THREE tests were failing — the awk over `test result:`
   lines does not survive a multi-binary run. Now uses the cargo EXIT CODE plus
   a FAILED-line count. Earlier commits' exit codes were 0 so those gates held,
   but the quoted counts were never independently checked.

## Known broken

- Two java tests PINNED the defect they were meant to guard
  (`java_static_import_resolves_to_its_own_class` asserted
  `java·org.junit·Assert·assertEquals`). Corrected in `2aaf6a09`. Assume more
  tests encode current behaviour rather than intent.
- `resolved` still counts resolution-to-a-stub: 241,046 of 438,270. It
  overstates real linkage and must not be quoted as "resolved".
- `sensei.edge_resolution_class` classes are named for what they MEASURE, not a
  verdict. `name-collision-1` is NOT a defect queue — its head is `json` 1,600,
  `path` 483, external accessors.
- detected_patterns: 1,447 rows, `family` NULL on ALL, unchanged across a
  reindex that added 106,906 edges. Undiagnosed.
- 176,261 of 176,281 unresolved references come from markdown prose. Not a
  defect; needs its own class rather than dragging the headline down.
- `.unwrap_or("")` in `lib_package_of` preserved deliberately, tracked.
- `nodes.language` is last-writer-wins on the reference path (graph.rs lacks the
  `CASE WHEN EXCLUDED.resolved` guard its neighbours have).

## Traps

Run the gate with the daemon STOPPED. NEVER `git checkout <path>` to revert a
probe — it destroyed uncommitted work TWICE this session; use `cp` to /tmp and
verify the restore with an explicit grep. Run ONE probe at a time: chained
probes invalidated each other's restore anchors three times. rustfmt rewraps
lines between scripted edits, so multi-line anchors go stale — prefer `Edit`.
`make install-debug` ends with `cargo clean` plus a multi-GB DB dump, so every
deploy is ~10 minutes. DDL must be applied to BOTH `sensei` and `sensei_test`.
