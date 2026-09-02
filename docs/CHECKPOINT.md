# Checkpoint

**Slice:** doc references now RESOLVE (`6bee2674`) — 0 → 74,427 (30.8%),
verified on a full reindex of 51,971 files. Branch `develop`.

## Graph-wide edge resolution

| kind | edges | resolved |
|---|---|---|
| `calls` | 336,682 | 65.1% |
| `references` | 241,515 | **30.8%** (was 0) |
| `imports` | 162,691 | 15.8% — **every local one**; rest genuinely external |
| `extends` | 7,901 | **0%** — next (#147) |

## Why references could never resolve

`extract_file_refs` returned `repo.join(spec)`, so a doc pointing at
`crates/mcp/src/lib.rs` was stored **machine-absolute** — while 98.9% of
`nodes.file_path` are repo-relative. They could never match. It now stores the
repo-relative form (existence still checked against the absolute one), which also
keeps a local filesystem layout out of a shared graph. And nothing tried to
resolve them at all — the same shape as the imports defect.

**Ambiguity is not resolved, deliberately.** A doc writes `` `handleAuth` `` with
no signature and no module path; with two same-named definitions there is nothing
to choose on. `sole_definition_id_by_name` returns `None` for ambiguous *and*
absent alike — a doc silently attached to the wrong same-named symbol is worse
than one attached to nothing. `LIMIT 2` is the trick: one row is unambiguous, two
means stop counting.

**A broken link mints nothing** — unlike an import, a file reference never
get-or-creates. A doc naming a nonexistent file is a fact about the doc.

Remaining unresolved, all correct: 121,510 bare words (ambiguous, or not symbols
— `true`, `id`), 29,767 SHOUTY_CASE env vars, 15,811 links outside the index.
Zero resolved to a stub or lib node. Spot-checked as real traceability:
`local-agent-coordinator.md` → the `run_status` enum DDL, the `advance_run`
handler, `playbook_run`, `driver_for`, `AcpObserveDriver`.

## Next — in order

1. **`extends` 7,901 / 0%** (#147) — small, and `codebase.rs:55` consumes it, so
   a live reader silently gets nothing.
2. **Externals → `lib_symbol`** — 136,642 edges. Largest, riskiest: mints nodes,
   MUST be lookup-first (59% of edges are locally-owned packages that only *look*
   external).
3. **46,117 unknown stubs** — all have in-edges so no GC touches them;
   java-dominated. Need resolution.
4. **TS/JS local callbacks** — `t` (573), `fn` (189), `setLoading` (235) need a
   locally-declared-names pre-pass.
5. Latent/cosmetic: `library_usage.unresolved_import_count` missing a kind
   filter; `libraries.rs` third classifier copy (inert); #150 content-hash
   identity (684 multi-folder path groups); 7,418 absolute `nodes.file_path`
   rows (1.1%); #149 community re-measure.

## Known-broken

`references` 251,229/0 (`doc_indexer.rs:584` and `:587` — two defects).
`extends` 7,901/0 (#147). `calls` 57% for this project.
`dojo_memberships.sync_status` dead. `graph-end-state-sketch.md` §1–12 NOT SAFE
TO BUILD FROM. 44,689 `unknown` stubs all have in-edges, so they need resolution
not GC (java 27,967 = 51%).

## Traps

**Never `git checkout -- <path>` to undo a mutation** — I did, and destroyed all
uncommitted work in `process.rs` (emit branch, hoisted `fqn_lang`, two tests).
Take a `cp` backup to /tmp before mutating and restore from that. Related: that
same bad mutation did NOT fail its test, which exposed a real gap — nothing
pinned lookup-first, since probe and get-or-create reach the same node when the
target already exists. A mutation that fails to fail is a finding.

**Run the full gate with the daemon STOPPED.** `metrics_pipeline_end_to_end`
failed once at 711s under daemon CPU/DB contention (`blocked=1 running=1`), then
passed in isolation in 5s and in a 186s uncontended full run. Timing-sensitive,
not flaky-for-no-reason.

The leak hook blocks real home-directory paths in source comments — use
placeholder shapes in doc examples.

`sensei_test` has NO automated schema provisioning — a DDL change must be applied
to both DBs. Never pipe a gate through `| head` (SIGPIPE truncated a run and
masked a real `fmt --check` failure). An incremental scan skips unchanged files,
so verifying an indexer fix needs `delete from sensei.scan_state` for the folder
first. Wipe needs the daemon STOPPED; `/health` not `/api/health`.

## Cleanup lesson

A cleanup predicate narrower than the classifier it cleans up after is easy to
get wrong: I cleared `@/` and `~/` and MISSED `lib·$lib%`, catching it only by
re-checking the remaining `$`-prefixed `lib_package` names instead of declaring
done. Enumerate the predicate from the classifier's own local classes.
