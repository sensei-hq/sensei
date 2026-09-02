# Checkpoint

**Slice:** FULL REINDEX DONE — 51,971 files / 8,855 folders in ~30 min on the
binary built from HEAD. Every **local** import now resolves graph-wide. Branch
`develop`, last code commit `a3edfe9c`.

## Reindex result

Cleared `sensei.scan_state` entirely (replace-per-file) rather than wiping
folders — no data loss, no 12-minute cascade delete.

| | before | after |
|---|---|---|
| imports resolved | 21,153 / 162,689 (13.0%) | **25,786 / 162,691 (15.8%)** |
| references | 250,939 (0%) | **241,514** (0%) — 9,425 garbage edges gone |
| calls | 218,766 / 336,180 (65.1%) | 218,941 / 336,400 (65.1%) — untouched |
| extends | 7,901 / 0 | 7,901 / 0 — untouched |
| nodes / folders | 728,304 / 8,855 | 727,939 / 8,855 |
| communities | 46,372 | 44,760 (−1,612 phantom) |

**Every local import resolves.** By bucket across all 162,691 edges: RESOLVED
25,786 · external 136,642 (correct) · `$virtual` 263 (framework, correct) ·
**zero** relative / alias / rust-internal / `$lib` misses. The 25,786 matches the
pre-fix local count of ~25,693, which is what confirms the plateau mid-run was
completeness rather than a stall.

**15.8% is the correct ceiling, not a shortfall** — 84% of imports here are
genuinely external, and a resolved edge would be *wrong* for them: the package
name in `target_name` is the answer, and it is the only place that string
survives.

The −9,425 references edges are the doc-extraction fix landing globally. That
kind is still 0% *resolved* because resolution for it was never written.

## The `@/` fix that was "already done" — and wasn't live

`804ef1fb` added the correct `@/` rule to `import_target` yesterday and wired it
to ONE consumer: the reporting endpoint `codebase.rs:138`. It touched **zero**
indexing files, so the classification was right for a day while the code that
writes the graph read a different copy. `git log -S '"@/"'` on `typescript.rs`
returns nothing — that shadow never had the rule.

It filed the project's OWN symbols under fabricated packages: **2,527**
`lib_symbol`/`lib_package` nodes from `@/`, `~/` and `$lib`, e.g.
`lib·@/validation·…·createTenantDetailSchema`. `import_anchor` is now the single
owner of the local-module-vs-external-package decision; `typescript_fqn::
classify_import` is a pure shape conversion with no judgement.

`$app`/`$env`/`$kavach` still anchor **externally** on purpose — those virtual
modules are framework-*provided*, so no local file can exist for them.

**Verified then cleaned:** one folder first (base-app-webapp 512 files, 181 → 0),
then all 2,527 deleted and both roots reindexed — OmniRoute re-walked 9,822 files
and produced **zero**. A reindex alone would not have cleared them: nothing
deletes lib nodes (`prune_file_nodes` filters `file_path=$2`, which is NULL for
them; `prune_orphan_stubs` excludes lib kinds by design). Only `$app`/`$kavach`
remain, correctly.

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

## Next — in recommended order

1. **`references` resolution** — 241,514 edges at 0%, the largest broken kind,
   and **now unblocked**. I previously called it blocked on a format mismatch;
   measured, that was overstated: 666,511 of 673,929 node `file_path` values
   (98.9%) are repo-relative, so normalise doc-ref targets to repo-relative and
   reuse the existing fqn/file lookup. The 7,418 absolute `file_path` rows (1.1%)
   are a separate small inconsistency.
2. **`extends` 7,901 / 0%** (#147) — small, and `codebase.rs:55` consumes it, so
   a live reader silently gets nothing.
3. **Externals → `lib_symbol`** — 136,642 edges. Largest count, riskiest: mints
   nodes, and MUST be lookup-first because 59% of edges (Java/Kotlin/
   Python-absolute/C) are locally-owned packages that merely *look* external.
4. **46,117 unknown stubs** — all have in-edges, so no GC touches them;
   java-dominated. Need resolution.
5. **TS/JS local callbacks** — `t` (573), `fn` (189), `setLoading` (235) need a
   locally-declared-names pre-pass.
6. Latent/cosmetic: `library_usage.unresolved_import_count` missing a kind
   filter; `libraries.rs` third classifier copy (inert); #150 content-hash
   identity (684 multi-folder path groups); #149 community re-measure.

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
