# Checkpoint

**Slice:** local import resolution DONE and verified (`cc534c63`), after the
stub-GC ordering fix (`a7a1bee9`) and the MCP shape fixes (`93cef04a`). Branch
`develop`. Gates: rust 2,689/0, app 1,698/118, clippy 0, fmt clean.

## Imports resolved 0 → 1,911

0 of 162,690 `imports` edges resolved — not because resolution failed but
because nothing tried: `process.rs` passed `target_id = None` unconditionally.
Twelve lines below, call edges reach 65% at the SAME emit site by get-or-creating
their target by FQN. It was one missing branch at one emit site.

**Both my initial framings were wrong**, refuted by a 26-agent investigation:
there is no resolution *pass* to add a kind-filter to (`TaskKind` has no
`ResolveEdges`; `resolve_edge` has zero production callers), and the ResolveLibs
barrier is the wrong home (the watcher re-inserts imports with NULL on every
edit, so a barrier fix is erased on the next keystroke).

Resolution is at emit and order-independent: a miss creates a stub on the
target's own fqn, and the target's later definition enriches that row in place
keeping its id. **Lookup-first** via the new non-mutating `node_id_by_fqn` —
get-or-creating on candidate 1 would satisfy candidate 1 forever and hide the
real target at candidate 2. It is folder-scoped, which matters because 5 repos
here have two checkouts each.

**Measured on the shipped artifact** (sensei project, forced reindex):
imports **0/3652 → 1911/3652**; remaining are 1,147 external (correct — the
package name is the answer, and a resolved edge has a NULL `target_name` so it is
the only place that string survives), 523 rust-internal (staged), 71 `$app`/`$env`
(framework, no local file). **Zero** unresolved relative/`$lib`/`@/`/`~/` edges,
against a predicted 86%. `graphHealth` reports imports at 52% where it read 0%.

## Next

1. **Rust `crate::`/`super::`/`self::`** — 1,151 edges. Needs `rust_lang`'s
   `classify_segments`/`parent_mod` arithmetic hoisted to `import_target`;
   `local_import_candidates` returns empty for `Internal` today, leaving those
   edges honestly unresolved.
2. **Externals → `lib_symbol`** — 136,997 edges. MUST be lookup-first: 59% of
   edges (Java/Kotlin/Python-absolute/C) are locally-owned packages that merely
   look external, and a local hit must win.
3. **Two shadow classifiers found by the investigation.** `typescript.rs:668-687`
   defines a second `classify_import` calling every non-dot specifier external;
   `libraries.rs:62-119` is a third drifted copy that records `@/lib/foo` as an
   external library named `@/lib`. Route both through the owner.

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

`sensei_test` has NO automated schema provisioning — a DDL change must be applied
to both DBs. Never pipe a gate through `| head` (SIGPIPE truncated a run and
masked a real `fmt --check` failure). An incremental scan skips unchanged files,
so verifying an indexer fix needs `delete from sensei.scan_state` for the folder
first. Wipe needs the daemon STOPPED; `/health` not `/api/health`.
