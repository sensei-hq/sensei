# Checkpoint

**Slice:** doc-reference extraction fixed (`a3edfe9c`); before that, ALL local
import classes resolve — rust use-paths (`d910c291`)
on top of the shadow-classifier retirement (`8035ec3e`), local imports
(`cc534c63`), the stub-GC ordering fix (`a7a1bee9`) and the MCP shape fixes
(`93cef04a`). Branch `develop`. Gates: rust 2,693/0, app 1,698/118, clippy 0,
fmt clean.

## Doc references: three extractor defects

`extract_file_refs` scanned prose OUTSIDE backticks (its sibling
`extract_fn_mentions` filters `i % 2 == 1` twelve lines away), and `Path::join`
REPLACES the path when its argument is absolute — so `repo.join("/")` is `/`,
which exists and got inserted: 2,038 live edges targeting literally `/`.
`extract_fn_mentions` also took any all-alphanumeric token, so 4,342 edges
targeted things like `429`; rejected now by a leading-digit rule, which is the
language's own identifier rule rather than a denylist.

No keyword/env-var denylist: `true`/`PORT`/`JWT_SECRET` still extract, and the
honest filter for them is resolution — they will not match a node.

**Measured** (sensei, forced reindex): `/` 204 → 0, bare numbers 22 → 0.

**Correcting my own framing:** I had bucketed all `target_name LIKE '/%'` as
garbage. Most is not — `extract_file_refs` RETURNS the joined absolute path by
design. Only `/` and near-`/` were the bug.

**And that surfaces why `references` is 0% resolved, which is NOT these three
defects:** file-ref targets are absolute while node `file_path` is mostly
repo-relative, so they cannot match — and `file_path` is not even internally
consistent (`.env.dev` alongside an absolute path to `app/`). That is the next
slice and needs its own measurement.

## Rust use-paths: imports 1,911 → 2,434

`local_import_candidates` returned empty for `ImportTarget::Internal`, so 1,151
`crate::`/`self::`/`super::` edges stayed unresolved. `classify_segments` already
had the arithmetic — including the leading-`super` up-count fold whose comment
records what a second copy cost before (`tasks::handlers::super::executor`, a
module that never existed) — so the pure half became
`rust_lang::internal_use_module` and `classify_segments` now calls it.

Rust needs two candidate SHAPES, not two module paths: `use crate::db::pg_store`
names either the module `db::pg_store` or an item `pg_store` in module `db`, and
only the graph knows which. The lookup-first probe picks whichever exists.

**Measured:** +523, exactly the rust-internal count; that bucket is now empty.
Remaining unresolved are 1,145 external, 71 `$app`/`$env`, 2 `$kavach` — all
correct, since those virtual modules are framework-*provided*. `graphHealth`
imports across the three slices: **0% → 52% → 66%**.

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

## Next

1. **Externals → `lib_symbol`** — 136,997 edges. MUST be lookup-first: 59% of
   edges (Java/Kotlin/Python-absolute/C) are locally-owned packages that merely
   look external, and a local hit must win.
2. **`references` still 0% resolved** — extraction is fixed, but the
   absolute-target vs repo-relative-`file_path` mismatch remains, and
   `nodes.file_path` is itself inconsistent. Measure that before resolving.
3. **`library_usage.unresolved_import_count`** (`library_usage.ddl:9-14`) counts
   unresolved edges of ANY kind, so its name over-promises. LATENT, not live: all
   9 matching edges are `imports`, because an unresolved external call resolves
   to a `lib_symbol` and never satisfies `target_id IS NULL`. One-line fix.
4. **`libraries.rs:62-119`** is still a third copy of the import filter (no `@/`
   or `~/` skip) — but INERT: its output goes to `folders.props.libs`, not to
   `sensei.libraries`/`referenced_libraries` (both zero alias rows), and
   `props.libs` is the clobbered-and-unread field from `95e23315`. Cosmetic;
   route it through the owner when convenient. (I earlier repeated an agent's
   claim that it produced live wrong libraries — it does not.)

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
