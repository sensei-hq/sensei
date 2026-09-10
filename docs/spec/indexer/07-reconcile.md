# Stage 7 — reconcile: dirty is not deleted, deleted is not demoted

Whole-system spec: `docs/design/indexer-v2.md` §7b in full — **R10.1, R10.3,
R10.4, R10.5, R10.6, R10.7, R10.8, R10.9** — plus A8, A9, D8, D9. Depends on
stage 6.

> **R10.2, R10.6's older three-clause form, and the pre-correction D8 are
> SUPERSEDED.** They prescribe DEMOTION, which was rejected. Read §7b before
> anything else in R10; the corrected rule is R10.7 (dirty) and R10.8
> (delete-and-unresolve).

## 1. Purpose

After a file is re-read, make the graph record exactly what that file now
states — no more, no less — without touching any other file's statements.

This is the stage where a naive implementation destroys data, and three
distinct wrong implementations all pass the obvious test.

## 2. Inputs and outputs

    diff_claims(previous: &BTreeSet<Fqn>, current: &BTreeSet<Fqn>) -> ClaimDiff   PURE
    reconcile(store, folder_id, file_id, facts) -> Result<Reconciled, _>          IO

    struct Reconciled {
        claimed, released, deleted, inbound_unresolved,
        contested, occurrences_dropped, edges_deleted,
    }

Exhaustively destructured at every call site (R9). Note `deleted` and
`inbound_unresolved` where an earlier draft had `demoted` — the rename is the
correction, not a cosmetic change.

## 3. Requirements

### The two events, two mechanisms

- **S1** (R10.7). An UNPARSEABLE file **removes nothing**. Its `files` row gets
  `skip_reason` plus the detail column stage 0 added, and its existing nodes
  are DIRTY — derived by joining the node to its file, never a flag written
  onto each symbol. Node, edge and claim counts are IDENTICAL before and after.
- **S2** (R10.3). A failed read never reaches reconcile, BY TYPE. `read`
  returns `Result<FileFacts, ReadError>`, so reconcile takes the facts of a
  successful parse and there is no other way to obtain them. Guard test:
  `FileFacts` has no production constructor outside a language module's `read`,
  and nothing calls `unwrap_or_default()` / `ok()` on a `Result<FileFacts, _>`.
- **S3** (R10.8). A declaration the file no longer makes is DELETED — detected
  by diffing the previous claim set against the current one, which is the
  detector demotion never had.

### Ordering — this is where data is destroyed

- **S4** (R10.8 clause i). **Unresolve before delete, same transaction.**
  `UPDATE edges SET target_id = NULL WHERE target_id = $z` runs BEFORE
  `DELETE FROM nodes WHERE id = $z`. Every FK is `ON DELETE CASCADE`, so a
  plain delete takes the inbound edges instead of unresolving them — the exact
  opposite of what R10.8's table requires.
- **S5** (R10.8 clause i). Assert `target_name IS NOT NULL` on every such edge
  BEFORE the update. Once the node is gone there is nothing left to recover the
  name from.
- **S6** (R10.8 clause ii). Assert the node has NO children via `parent_id`
  immediately before the delete. Members of a deleted type are declared by the
  same file and released by their own claims, so the cascade must find nothing.
  If it ever fires, it has silently taken a member's inbound edges.

### Attribution

- **S7** (R10.1). The occurrence key is the ONLY edge-side unit. Use
  `props->'occurrences' ? F` against stage 0's gin index. **Do NOT narrow it.**
  The committed `v2_edges_contributed_by` adds
  `AND (s.fqn = ANY($current) OR s.resolved = false)`, so an edge whose source
  this file deleted, and which resolves elsewhere, is never revisited and its
  stale occurrence survives forever. Scoping by folder is enough: **19.7ms
  measured against the largest folder here (330,437 edges).**
- **S8** (R10.1). Never `DELETE FROM nodes WHERE file_path = $F`. Measured:
  **356 of 372 files have their own module node declared by ANOTHER file**, and
  **3,290 file-scope edges across 355 files** are sourced at a node whose path
  names a different file. That predicate — what `delete_nodes_by_file` does
  today — deletes every child module node of a `lib.rs` and cascades all 3,290
  edges away, in silence, from files nobody re-indexed.
- **S9** (R10.4). Remove only F's key from `props.occurrences`; delete the edge
  row iff the result is `{}`. Never the source-scoped
  `DELETE FROM edges WHERE source_id = ANY(…)`, which is the v1 shape and takes
  every other file's occurrences with it.
- **S10** (R10.4, D9). A node is deleted only when `props.claims` is EMPTY.
  When a claim remains but the released one owned the definition columns, clear
  those columns and keep the row — it awaits refresh from the surviving
  claimant. That is the ONLY case leaving cleared definition columns, and it is
  not demotion under another name, because the node still has a claimant.
- **S11** (R10.4). Two files claiming one identity is an A7 VIOLATION, not a
  state to support. Report it in `contested`; the repair belongs to the
  identity rule.

### Other cases

- **S12** (R10.3). The brake: if F previously claimed ≥1 declaration and now
  claims zero, re-read from disk ONCE and apply only if the second read also
  claims zero. A file ABSENT from disk skips the brake — that is OBSERVED, not
  inferred.
- **S13** (R10.5). A deleted file is `reconcile(F, claims = ∅)`, the same code
  path. A folder deletion is N file reconciles, never a path-prefix `DELETE`.
- **S14** (R10.5). A rename is a deletion plus an addition and needs no case.
  Do not write rename inference.
- **S15** (R10.9, A9). The parse failure must be ACTIONABLE and REACHABLE: the
  parser's verbatim message with file, line and column; an MCP query for "which
  files in this project are unparseable, and why"; and every query returning a
  dirty node LABELS it dirty with the reason. **The MCP surface and the
  query-labelling are real work in this stage, not a byproduct** — a dirty flag
  nothing can query is invisible and therefore useless.

## 4. Failure modes

| input | this stage does |
|---|---|
| `read` returned `Err` | reconcile does not run at all (S2). The file keeps the graph it had; the failure is recorded (S1). |
| the file claims zero and previously claimed some | brake (S12), then apply. |
| the file is absent from disk | apply immediately, no brake (S12). |
| a node to delete has children | fold them in or fail (S6). Never let the cascade run. |
| an inbound edge has NULL `target_name` | STOP and report. Deleting loses the reference irrecoverably (S5). |
| two files claim one identity | `contested`, reported, not resolved (S11). |
| a DAMAGED file that still parses | undetectable, and accepted. Half its declarations are removed, and the loss is RECOVERABLE: the next healthy index of the SAME FILE restores them and the unresolved edges — `target_name` intact — re-resolve. No other file must be re-indexed. |

## 5. Verification

The mutation fixture is a temp git repo the test builds and mutates. Specified
HERE and referenced by stage 9; not duplicated.

| case | assert | mutation that must break it |
|---|---|---|
| **R10.6(a)** | claims of F equal `facts.symbols`; `occurrences -> F` equals the grouping of facts | any partial write |
| **R10.6(b) — LOAD-BEARING** | index two files, re-index one, the OTHER's contribution is byte-identical before and after | write it against THREE deliberately wrong impls and watch it catch each: `delete_nodes_by_file`-style node deletion, `DELETE FROM edges WHERE source_id = ANY(…)`, and dropping the whole `occurrences` object. (a) passes for all three. |
| **R10.6(c)** | for every fqn in `previous_claims(F) \ facts.symbols`: NO node exists, AND every edge that targeted it has `target_id IS NULL` with non-null `target_name` | remove either half — the node still existing is the demotion defect, the edge being gone is the cascade defect |
| **R10.6(d)** | an unparseable file changes NOTHING: node, edge and claim counts identical | make an unparseable file take the empty-facts path |
| delete symbol | remove `z()` from `z.rs`; z's node DELETED, x's edge UNRESOLVED with `target_name` kept, NOT pointing at a file-less node | skip S4's unresolve |
| cascade guard | the deleted node's inbound edge count before equals `inbound_unresolved` after | reorder S4 after the delete |
| child guard | deleting a type deletes its members via their own claims, and `children_at_delete` is 0 | rely on the `parent_id` cascade |
| parent/child edges | re-index a `lib.rs`; the child files' 3,290-shaped file-scope edges survive | use the `file_path` predicate (S8) |
| shared edge | two-file fixture minting one source identity; releasing one keeps the other's occurrence | the corpus has only 4 such edges, all from one A7 identity, so it CANNOT carry this test — the fixture is mandatory |
| shared claim | two files declaring one fqn; releasing one must NOT delete the node | make deletion unconditional on release |
| narrowed lookup | an edge whose source this file deleted, resolving elsewhere, IS revisited | restore the `AND (s.fqn = ANY(...))` narrowing |
| unparseable switch | a good file becomes unparseable; nodes go DIRTY, nothing is removed | treat it as empty |
| rename | `a/b.rs` -> `a/c.rs`; old fqns gone, new ones present, no rename code exists | add rename inference |
| delete file | `reconcile(F, ∅)` takes the same path as an emptied file | give deletion its own path |
| move | same fqn in two files, BOTH scan orders, identical graph (A6) | require release-before-claim |
| brake | ≥1 claims -> 0 re-reads once; absent-on-disk does not | brake on the absent case |
| idempotence | reconcile twice with the same facts changes no row the first pass did not | any non-idempotent write |
| **A9** | MCP answers "which files are unparseable and why"; a dirty node comes back LABELLED | implement the state and skip the surface |

## 6. Watch out

**`props || EXCLUDED.props` vs `jsonb_set(props, '{k}', v)`** — the first
replaces `claims`/`occurrences` wholesale. Every bug in this stage is a variant
of that one.

**Do not gate on `tree.has_error()`.** Measured wrong in both directions: true
for 6 of 372 intact files, false for 53 of 366 truncated ones. It would refuse
to ever reconcile six healthy files and wave through fifty-three damaged ones.

**Do not add a GC.** `prune_orphan_stubs_scoped` already owns that predicate
and already runs every tick.

**A package rename is NOT this stage** (R10.5). Reconcile handles the renamed
package's own files correctly; what it cannot reach is every reference to that
package from every OTHER package, which lives in files the rename did not
touch. The trigger is a folder-level observation by the manifest reader, and
the action is a full re-index of the scan root. Do not add a partial version.

## 7. Definition of done

- `diff_claims` is pure and unit-tested.
- All four R10.6 clauses hold over the corpus, including (d).
- R10.6(b) is proven by catching three wrong implementations.
- The unresolve-before-delete ordering is asserted by an edge-count check, not
  only a node-count check.
- `children_at_delete` is 0 across the corpus.
- The narrowing in `v2_edges_contributed_by` is GONE.
- A9's MCP surface exists and returns real unparseable files.
