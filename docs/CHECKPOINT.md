# Checkpoint

**Slice:** per-language indexer gap remediation. Branch `develop`, issue #130.
Plan: `docs/spec/2026-09-02-capability-refactor-map.md` + the per-language issue list.

## Done

- **Slices 2/3/4 — DEPLOYED and verified live.** Inheritance 0 → 2,677 edges at
  99.8%. Imports 18.9% → 100.0% (136,562/136,573), survived the stub prune.
- **`78828f7d` #1** TS/JS global receiver → runtime lib. NOT deployed (~12,371 eligible).
- **`2aaf6a09` #2** java third-party CALL → lib node. NOT deployed (~17,209 fabricated).
- **`b9818af5` #2b** java third-party SUPERTYPE → lib node. NOT deployed. Found this
  round: #2 left `resolve_supertype` on the `is_external_pkg` JDK allowlist, so one
  adapter answered two ways about `org.springframework` — lib when CALLED, fabricated
  first-party when EXTENDED. 125 live edges. Both paths now share `is_first_party`;
  the allowlist is deleted.

## Remaining, in dependency order

1. **#5** `walk_stmt` (typescript.rs:738) is a flat single-level match. Likely FIRST:
   #3's local-def probe reads the def set #5 changes. Same file as #3.
2. **#3** bare-identifier fabrication (typescript.rs `resolve_name` None arm, ~1167).
3. **#4** kotlin emits no refs/relations (kotlin.rs). Independent of the above.

Then ONE reindex measures #1, #2, #2b and #3 together — they share an emit path.

## Next command

`cargo test -p senseid --bin senseid languages::` — 3.9s, no DB, no daemon-stop.
(`--lib` fails: senseid has no lib target.)

## Measured myself this round — these supersede older quotes

- #3: 21,981 phantom TS/JS function nodes (ts 11,562 + js 10,419) absorb 25,696 call
  edges vs 33,547 real defs — 41% of TS/JS function call edges hit a node with no
  definition anywhere.
- #4: kotlin has `imports|3713` and zero other edge kinds across 2,062 def nodes.
- #5: test files are 22% of TS/JS files but yield 7.0% of call edges (5.2 vs 19.8/file).
- Headline unchanged: real def-to-def call linkage is 73,792/340,229 = **21.7%**, not
  65.2%. The defect is INVENTION, not absence.

## Known broken / do not repeat

- `resolved` counts resolution-to-a-STUB (241,046/438,270). Never quote as "resolved".
- `edge_resolution_class` names what it MEASURES, not a verdict. `name-collision-1` is
  not a defect queue — its head is `json` 1,600, `path` 483, external accessors.
- `detected_patterns`: 1,447 rows, `family` NULL on ALL, unchanged across a reindex
  that added 106,906 edges. Undiagnosed.
- 176,261 of 176,281 unresolved refs are markdown prose. Needs its own class.
- The 40,651 duplicates are a FOLDER-REGISTRY defect (one repo at two paths; `cluster`
  vendoring `server`/`scheduler`), NOT an fqn-scheme defect. Refuted twice — drop it.
- `nodes.language` is last-writer-wins on the reference path (graph.rs lacks the
  `CASE WHEN EXCLUDED.resolved` guard its neighbours have).
- Assume tests PIN current behaviour, not intent — two java tests already did.
- `import_target.rs` deliberately probes the graph rather than testing any prefix. Do
  NOT "fix" it to use `is_first_party`; the probe is the stronger question.

## Traps

Gate with the daemon STOPPED for the FULL suite (`languages::` alone needs neither).
NEVER `git checkout <path>` to revert a probe — destroyed uncommitted work twice; `cp`
to /tmp and verify with an explicit grep. ONE probe at a time. rustfmt rewraps between
scripted edits, so prefer `Edit`. `PIPESTATUS` is bash — in this zsh it is silently
EMPTY, and a backgrounded chain reports the LAST command's code, not cargo's; redirect
and read `$?` unpiped. `make install-debug` ends with `cargo clean` + a multi-GB dump,
so every deploy is ~10 min. DDL goes to BOTH `sensei` and `sensei_test`.
