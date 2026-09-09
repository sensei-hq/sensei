# Build plan — indexer v2, rust

Spec: `docs/design/indexer-v2.md`. Requirements are cited as R1..R10, acceptance
as A1..A8. This document is only the sequence: what to build, how to prove it,
what goes wrong.

Rules for every step: failing test first, run it, watch it fail. Gate before
moving on — `cargo fmt --all`, `cargo clippy --workspace --all-targets -D
warnings`, `cargo test --workspace`. No step is done while the next one is
started.

---

## Step 1 — Fact types

**Build** `crates/senseid/src/indexer/facts.rs`. Types only, zero logic:
`FileFacts`, `Symbol`, `SymbolKind`, `Reference`, `RefKind`, `Resolution`,
`Reason`, `Evidence`, `Relation`, `Import`, `Fqn`, `Span`.

**Verify**
- A test constructs one value of every enum variant.
- A test asserts `Resolution` has exactly two variants and neither is empty.
- `rg 'Option<Fqn>' crates/senseid/src/indexer/` returns nothing.

**Watch out**
- `Reason` must be an enum, not a `String`. A string reason cannot be
  exhaustively matched and the histogram silently grows junk categories.
- No `Option` may stand for "unresolved" anywhere in these types (R2).
- Do not add convenience constructors that default a field. A defaulted field is
  how fabricated data enters (R4).

---

## Step 2 — FQN construction

**Build** `crates/senseid/src/indexer/fqn.rs`. The grammar from spec §2, one
builder per form.

**Verify**
- For a table of `(package, module, type, member)` inputs, assert the
  DEFINITION-side builder and the REFERENCE-side builder produce byte-identical
  strings. This is the merge contract; if it fails nothing else matters.
- Assert empty segments are dropped identically in both.
- Round-trip: parse a built fqn back into parts.

**Watch out**
- Two call sites building an fqn by hand instead of calling the builder is how
  the sides drift apart. There must be no `format!("{}·{}", ..)` outside this
  file — enforce with a test that greps the tree.

---

## Step 3 — Rust walk: declarations

**Build** `crates/senseid/src/indexer/lang/rust.rs`, declarations only. Emit a
`Symbol` for every function, method, struct, enum, trait, type alias, const,
module, **field, tuple-struct field, enum variant**. Parameters are typed props
on their function (D2). Record the declared type for fields and returns (R3).

**Verify**
- Fixture file with one of each. Assert symbol count equals a hand-written
  expected count, and assert each expected fqn is present.
- Independent counter: a second tiny walk that counts declaration nodes by
  tree-sitter kind. Symbol count must equal it. Not a self-check by the
  producer.
- Assert a field and a method with the SAME name on the same type produce
  distinct fqns and distinct identities.

**Watch out**
- `nodes_unique_identity` is `(folder_id, file_path, kind, name, parent_id,
  line_start)`. A field and a method of the same name at the same line collide
  unless `kind` separates them. Test it explicitly.
- A tuple-struct field has no name. Decide its identity (index) and write it
  down in the test.
- Do not emit parameters as symbols (D2).

---

## Step 4 — Rust walk: references, all of them

**Build** reference emission. Every call, member access, path use and
construction yields exactly one `Reference`. Resolution may be `Unresolved` —
omission is not permitted (R2).

**Verify — this is the load-bearing check (A2)**
- Write an independent counter that walks the same tree and counts use sites by
  tree-sitter kind, with no knowledge of the resolver. Assert
  `references.len() == that count`, over this repo's own rust sources, not a
  fixture.
- Deliberately malformed source (`x.();`, `().0();`, unterminated) must not
  panic and must not silently drop.

**Watch out**
- A catch-all match arm returning early is the exact defect this rewrite exists
  to remove. There must be no arm that yields nothing; the fallback yields
  `Unresolved { reason: UnhandledForm, .. }` carrying the node kind so the
  histogram names it.
- Macro invocations are a distinct tree-sitter kind from calls. Decide whether
  they are references and record the decision; do not let them fall through
  unnoticed.

---

## Step 5 — Resolution ladder and reason codes

**Build** `crates/senseid/src/indexer/resolve.rs`. Shared across languages (R7).
Local declarations, then imports, then external classification. Everything else
`Unresolved` with a specific reason.

**Verify**
- One test per `Reason` variant, asserting that exact variant is produced.
- Histogram test over this repo's rust: every unresolved reference has a reason
  and the reasons sum to 100% (A3).
- Order independence (A6/R6): resolve the same file with an empty "already
  scanned" set and with a full one — identical output.

**Watch out**
- Never infer externality from absence. A symbol missing from the current file
  set may simply not be scanned yet; that makes the graph depend on scan order.
  Externality comes from the import (spec §2).
- The denylist (plumbing like `clone`, `unwrap`) is filtering, not failure. Give
  it its own reason so a reader can exclude it without losing genuine misses.

---

## Step 6 — Relations

**Build** `extends`, `implements`, impl blocks, trait impls, member ownership,
from the same walk (D5).

**Verify**
- Fixture with an inherent impl, a trait impl, and a generic impl. Assert the
  inherent impl produces NO relation and the trait impl produces one.
- Assert every relation's child and parent are resolvable or explicitly
  unresolved — never a bare name with no reason.

**Watch out**
- An inherent `impl Foo { }` is not inheritance. Emitting one is a false edge
  that pattern detection will read as real.

---

## Step 7 — Persistence

**Build** the emit path from `FileFacts` to the database (R3, D1). The wire
format carries everything the walk captured, including declared types and
unresolved reasons.

**Verify**
- Round-trip: index a fixture file, read the rows back, assert every field of
  every `Symbol` and `Reference` survived. Specifically assert a declared type
  reaches the database — that is the seam that lost data before.
- Assert an `Unresolved` reference produces a row, not an absence.

**Watch out**
- A positional-argument insert with no slot for a new field silently drops it.
  Use a struct, not positional args.
- A definition arriving after a reference must merge onto it, not create a
  second node (spec §2, the merge contract).

---

## Step 7b — Reconcile

Numbered 7b and not 8 on purpose: `persist.rs` and the spec cite "step 9" for
the DDL decisions in a dozen places, and renumbering would falsify all of them.

**Spec** `docs/design/indexer-v2.md` R10 (R10.1–R10.6), A8, D8, D9. Read it
first — the eight edge cases are settled there and the numbers behind each are
in it. This step is the sequence only.

**Build** `crates/senseid/src/indexer/reconcile.rs`, plus the v2-only store
functions it needs. Nothing under `crates/senseid/src/languages/` and no change
to any pre-existing `pg_store` function; `upsert_v2_symbol`,
`merge_v2_edge_occurrences`, `v2_definition_nodes` and `v2_edges` are v2's own
and may change.

Build in this order — each item is usable and testable before the next exists.

1. **Claims (D9).** `upsert_v2_symbol` also writes
   `props = jsonb_set(props, '{claims}', coalesce(props->'claims','{}') ||
   jsonb_build_object($file, true))`. NOT `props || …` — that replaces the whole
   `claims` object, which is the exact bug `merge_v2_edge_occurrences` exists to
   avoid. Add `PgStore::v2_claims_of_file(folder_id, file_path) -> BTreeSet<String>`.
2. **Demotion (R10.2).** `PgStore::demote_v2_symbol(node_id)`: clear
   `file_path`, `line_start`, `line_end`, `docstring`, `signature`, set
   `resolved = false`, `is_exported = false`, `kind` to the placeholder
   `persist::KIND_NOT_YET_KNOWN` already defines, and REMOVE the props keys
   `symbol_kind`, `visibility`, `declared_type`, `params`, `span_columns` (jsonb
   `-`, not a whole-props write — `claims` and anything another writer owns must
   survive). A left-behind `line_start` on a `resolved = false` row is a
   fabricated definition site (R4).
3. **Occurrence removal (R10.4).**
   `PgStore::drop_v2_edge_occurrences(edge_id, file_path)` →
   `props = jsonb_set(props, '{occurrences}', (props->'occurrences') - $2)`,
   then delete the edge row iff the result is `{}`. Return which of the two
   happened; the caller counts both.
4. **The edge lookup (R10.1).**
   `PgStore::v2_edges_contributed_by(folder_id, file_path, claims)` with the
   two-disjunct predicate R10.1 states. Assert over the corpus that it is a
   SUPERSET of `props->'occurrences' ? file_path`, never a narrowing.
5. **`reconcile(store, folder_id, facts) -> Reconciled`.** Typed return,
   destructured exhaustively (R9): `{ claimed, released, demoted, contested,
   occurrences_dropped, edges_deleted }`. `contested` is the R10.4 report — an
   identity another file still claims — and exists for the same reason
   `persist::Written` returns `collisions`.
6. **The read boundary (R10.3).** A `Read` outcome that reconcile consumes, so
   an `Err` from `lang::*::read` cannot reach it as an empty fact set. Guard
   test over the v2 sources: `FileFacts` has no production constructor outside a
   language module's `read`, and nothing calls `unwrap_or_default`/`ok()` on a
   `Result<FileFacts, _>`.
7. **The brake (R10.3).** Previously ≥1 claim and now zero ⇒ re-read from disk
   once, apply only if the second read also claims zero. Absent-on-disk skips
   the brake — that is observed, not inferred.

**Verify** — (b) is the one that matters; (a) passes for three implementations
that destroy data.

- R10.6(a): claims of F equal `facts.symbols`; `occurrences -> F` equals the
  grouping of `facts`.
- **R10.6(b), the load-bearing check:** index two files, re-index one, and
  assert the OTHER file's contribution is byte-identical before and after —
  read back through `persist::read_back` with F projected out. Write it against
  three deliberately wrong implementations and watch it catch each:
  `delete_nodes_by_file`-style node deletion, `DELETE FROM edges WHERE
  source_id = ANY(…)`, and dropping the whole `occurrences` object.
- R10.6(c): every fqn in `previous_claims(F) \ facts.symbols` still has a node,
  `resolved = false`, `file_path = NULL`, no definition column or prop.
- Shared edge: a two-file fixture minting one source identity (the corpus has
  only 4, all from one known A7 identity, so it cannot carry this test).
- Shared claim: two files declaring one fqn; releasing one must not demote.
- Parent/child: re-index a `lib.rs` and assert the child files' 3,290-shaped
  file-scope edges survive. This is the case `file_path` attribution breaks.
- Deletion: `reconcile(F, ∅)` down the same path as an emptied file.
- Move: same fqn, two files, both scan orders, identical graph (A6).
- Idempotence: reconcile twice with the same facts changes nothing.

**Watch out**

- `props || EXCLUDED.props` and `jsonb_set(props, '{k}', v)` are not
  interchangeable. The first replaces `claims`/`occurrences` wholesale. Every
  bug in this step is a variant of that one.
- Do not add a GC. A stub with no edges is `prune_orphan_stubs_scoped`'s and it
  already runs every tick. Two owners of deletion is a race.
- Do not gate on `tree.has_error()`. Measured: true for 6 of 372 intact files,
  false for 53 of 366 truncated ones — wrong in both directions.
- Do not infer a rename. `a/b.rs` → `a/c.rs` changes every fqn in the file; it
  is a deletion plus an addition and needs no code.
- A package rename is NOT this step (R10.5). Do not add a partial version of it.

---

## Step 8 — Differential harness

**Build** a harness that runs v1 and v2 over the same corpus and diffs the facts.
Not a test — a tool that prints a report.

**Verify**
- Every difference is classified: improvement, regression, or explained. A
  regression blocks cutover.
- v2's resolved set must be a superset of v1's, or each exception justified in
  writing.

**Watch out**
- v1 drops references silently, so v2 will show far MORE references. That is the
  intended result, not a regression — compare resolved edges and node identity,
  not raw counts.

---

## Step 9 — Cutover, reindex, acceptance

**Build** the switch in the file processor for rust only. Three deletion
triggers must be re-pointed at `reconcile` in the same change, or the graph gets
two removal semantics: `process_git_folder`'s `plan.removed` loop,
`scan::prune_vanished`, and the `delete_file` / `delete_folder` handlers. All
three call `delete_nodes_by_file` today, whose `file_path` predicate deletes
child module nodes and cascades their edges (R10.1).

The DDL decisions this step owns, all deferred here by earlier steps: widening
`sensei.node_kind` (for `trait`/`static`/`macro` and a "not yet known" value
that retires the `parameter` placeholder), widening `sensei.edge_kind`, and
`create index … on sensei.edges using gin ((props->'occurrences'))`, which
collapses R10.1's two-disjunct lookup into one test.

**Verify** — run the acceptance list (A1..A8) against the live graph after a full
reindex. `sensei scan` will not pick up an indexer change; force it by resetting
`daemon.last_version` and restarting, then set it back.

**Watch out**
- Reindexing all 48,654 files takes ~20 minutes plus embedding backfill. A shell
  command may cap at 10 minutes — poll across several.
- Record the before numbers BEFORE deploying. They cannot be recovered after.

---

## Step 10 — Retire

Move the superseded rust module to `to_be_discarded/`. Only after step 9 passes.
Other languages follow the same ten steps, one at a time (D4).

---

# Pipeline decomposition (R14/R15) — the top-down shape

    scan_root(watch_root)                          TASK
      find_git_roots(path)                  IO     glob for .git directories
      -> enqueue scan_repo per root                            [queue]

    scan_repo(repo_root)                           TASK
      find_submodules(gitmodules_text)     PURE    declared, reliable
      find_subtrees(git_log_output)        PURE    heuristic, provenance=inferred
      find_files(root, filters)             IO     glob + apply filters
      save_repo(repo, submodules, subtrees) DB
      save_folders_and_files(entries)       DB     <-- STRUCTURE BARRIER (R14)
      -> enqueue index_file per file                           [queue]

    index_file(file_id)                            TASK
      read -> walk -> resolve -> persist -> reconcile

## Purity is the testability seam

The three `find_*` functions take TEXT or a PATH and return DATA. No database, no
task queue, no hidden state:

| function | input | testable with |
|---|---|---|
| `find_submodules` | `.gitmodules` contents | a string literal |
| `find_subtrees` | `git log` output | a fixture, including the KNOWN-BAD cases |
| `find_files` | a root + filter set | a tempdir |

`find_subtrees` in particular MUST be pure, because its heuristic is known to be
wrong in four distinct ways (§7g) and the only way to pin that behaviour is to
feed it recorded output and assert what it does with each. A version that shells
out to git internally cannot be tested against a rebased history it will never
see on this machine.

## Ordering is the correctness property, not a style choice

`save_folders_and_files` completes BEFORE any `index_file` is enqueued. That
barrier is what gives R14 its three properties — no task races another to create
a file row, the complete post-filter set IS `expected_files`, and a file row with
no parse outcome is a visibly lost task.

`save_repo` precedes it because a folder needs its repo, and files need folders.

## What each stage owns

| stage | owns | must not |
|---|---|---|
| `scan_root` | which repos exist | know anything about files |
| `scan_repo` | the file and folder set of ONE repo | parse anything |
| `index_file` | one file's nodes and edges | create a file row (R13) |

`index_file` looking up a `file_id` and failing closed — rather than
get-or-create — is what keeps that boundary real.

## Progress is emitted for the UI, and the barrier is what makes it determinate

`RepoProgress { total, pending, running, completed, failed, current_file }`
already exists (`tasks/progress.rs:38`) and is consumed by the UI at both file
and repo level. Two consequences of R14 that must not be optimised away later:

**`total` is known BEFORE the first file is indexed.** Because
`save_folders_and_files` completes before any `index_file` is enqueued, the
denominator is fixed at the barrier. Enqueueing as files are discovered would
make `total` grow during the scan, so the UI would show a percentage that moves
backwards. The barrier is what makes progress determinate rather than a spinner
with a number on it.

**Most of it is DERIVED from the files table, not tallied.** The file lifecycle
(§7f) maps directly onto the counters:

| counter | source |
|---|---|
| `total` | `count(files)` for the repo |
| `pending` | files discovered with no parse outcome |
| `completed` | files parsed |
| `failed` | files unparseable, by `skip_reason` |
| `running` | the TASK QUEUE — not derivable from files |
| `current_file` | the task queue |

Deriving four of the six means they cannot drift from the thing they describe —
the same argument as folder completeness (R10.7b) and node dirtiness (R10.7).
Only `running` and `current_file` are genuinely queue state, because "a task is
executing right now" is not a fact about a file.

This also means progress survives a restart. A maintained tally is lost when the
daemon stops; a derived one is recomputed from rows that are already durable.

**Repo-level progress needs `failed` to distinguish causes.** An unparseable file
and an excluded file are both "not indexed" but only one is actionable (R10.9).
The UI should be able to show "3 files could not be parsed" separately from "17
binary files skipped", because the first is a call to action and the second is
noise.

## Exclusions: PRUNE during the walk, never filter after it

A watch root carries exclusions (`folders_to_watch.excluded`, e.g. a
dev root excluding `["Code"]`). These must prune the traversal, not filter
its output.

Globbing everything and discarding afterwards pays the full descent cost on
exactly the trees you least want to enter — `node_modules`, `target`, `.venv`,
build output — which is where the file count explodes. The walker must be told
"do not descend here" at the directory, so the subtree is never read.

Two levels, both pruning:
- ROOT exclusions from `folders_to_watch.excluded`, relative to the watch root
- the shared default globs (`DEFAULT_EXCLUDE_GLOBS`), which already exist and
  already have one owner — do not grow a second list

`find_files` takes the filter set as an ARGUMENT (see the purity table), so the
pruning rule is testable against a tempdir without a watch root or a database.

## Accepted scope reduction: git-only roots

`scan_root` globbing for `.git` means a directory that is not a git repository is
no longer a root. Measured, that drops the `standalone` kind: 89 folders today,
against 84 `git`. Everything else is unaffected — the 9,076 `folder` rows are
subfolders INSIDE repos and are created by `scan_repo`, and 129
`workspace_member` rows likewise.

This is deliberate. A repo boundary is a real, declared thing (`.git`); "a
directory someone pointed us at" is not, and it is what produced the
nested-root and duplicate-project drift classes the doctor has to repair. If a
non-git directory genuinely needs indexing, the honest answer is to say so
explicitly rather than infer roothood from a walk.

## Incremental: files -> repos, the inverse traversal

The full scan goes repo -> files. A change arrives the other way:

    list_changed_files(watch_root)         IO     watcher or a diff
    match_repos(paths, known_roots)       PURE    group each path to its repo
    save_folders_and_files(changed)        DB     <-- SAME BARRIER
    -> enqueue index_file per changed file             [queue]

`match_repos` is pure — paths in, groupings out — so it is testable with string
literals and no filesystem. Three rules it must get right:

**LONGEST PREFIX WINS.** A file under `repo/submodule/x.rs` belongs to the
SUBMODULE, not the parent. Matching the first or shortest root attributes a
submodule's files to its parent repo, which silently merges two codebases.

**A path matching NO known root is not a file event.** It means a repo appeared,
or the path is outside every watch root. Enqueueing an `index_file` for it would
create work with no repo to attach to. It escalates to `scan_root`, it does not
guess.

**Some changed files are REPO events, not file events.** A modified
`.gitmodules`, `Cargo.toml [workspace] members`, `package.json workspaces` or
`sensei.library.json` changes the repo's STRUCTURE, so it re-runs the repo-level
discovery rather than being parsed as a source file. Treating them as ordinary
files leaves the structure stale while the content is current.

The barrier holds here too, for the same reason as R14: rows for the changed set
are written before any task is enqueued, so no task races another and the
denominator for progress is the changed count, known upfront.

Deletions produce NO task. A deleted file is reconcile's job (R10.8) — the claim
is released, the node is deleted, inbound edges are unresolved. Enqueueing a
parse task for a file that no longer exists is how a pipeline ends up with a
"file not found" error path it then has to interpret.

## .gitignore applies in scan_repo, and it is NOT the same list as the globs

Ignore rules are a REPO concern, so they belong in `scan_repo`, not `scan_root`
— at root time we do not yet know where the repos are, and a `.gitignore` only
has meaning relative to one.

Two filters, complementary, neither replacing the other:

| filter | says | examples |
|---|---|---|
| `.gitignore` | NOT TRACKED — the repo's own declaration | `target/`, `node_modules/`, `.env`, `.DS_Store` |
| `DEFAULT_EXCLUDE_GLOBS` | tracked, but NOT SOURCE | committed `*.min.js`, vendored bundles, `assets/*-<hash>.js` |

Git states what is not part of the repository. The glob list states what is in
the repository but is not worth indexing. Conflating them loses one or the other:
drop the globs and committed minified bundles get parsed; drop gitignore and we
maintain a list the repo already publishes.

### The current gap

The walks use `walkdir`, which is NOT gitignore-aware, so the repo's own
declarations are currently unread — including NESTED ones, of which this repo has
five (`/`, `database/`, `app/`, `website/`, `dojo/`). Nested matters: a
`.gitignore` deep in a tree can exclude a subtree that the root file says nothing
about.

`ignore = "0.4"` is ALREADY a dependency and is gitignore-aware, including
nested files and `.git/info/exclude`. `scan_repo` should walk with its
`WalkBuilder`, which prunes at the directory (satisfying the prune-not-filter
rule above) and reads the hierarchy correctly.

This is also why `DEFAULT_EXCLUDE_GLOBS` grew entries like `**/target/**` and
`**/node_modules/**`: they were hand-added to compensate for gitignore not being
read. Once it is, those entries are redundant and should go, leaving only the
tracked-but-not-source cases the list actually exists for.

---

# Verification plan — a checkpoint per layer, observable without asking

Each stage produces DATA before it enqueues anything. That is what makes every
layer verifiable on its own: the test asserts the returned value, no queue and no
next stage involved. It is also why the `find_*` functions must stay pure.

## A checkpoint per stage, written where it can be tailed

Every stage appends one JSON line to a progress file — default
`~/.sensei/scan-progress.jsonl` — so the run is observable live with
`tail -f` rather than by asking. One line per stage completion, plus a SAMPLE so
the shape can be eyeballed, not just the count.

| stage | the line carries | the sample proves |
|---|---|---|
| `scan_root` | roots found, excluded count | the git folders found, AND each exclusion with the rule that caused it |
| `scan_repo` | repo, folders, files kept/filtered | a few kept paths and a few filtered ones WITH the reason (gitignore / glob / not-source) |
| `index_file` | file, symbols, references, unresolved by reason | one file's nodes and edges in full |
| `reconcile` | claims released, nodes deleted, edges unresolved | the specific fqns, not just counts |

A count alone hides the interesting failure. "kept 412 files" is consistent with
having silently dropped every `.svelte`; "kept 412, filtered 88, here are 5 of
each with reasons" is not.

## Gate each stage BEFORE it enqueues

The order of work is the order of verification:

1. `find_git_roots` — two configured roots produce the expected git folders, and
   an excluded folder is absent WITH its rule named. Pure input, pure output.
2. `find_files` — one repo produces the expected kept/filtered split, nested
   `.gitignore` honoured, globs applied, prune-not-filter confirmed by the
   subtree never being read.
3. `save_folders_and_files` — rows land, the barrier completes, `expected_files`
   equals the kept count.
4. `index_file` — one file yields the expected symbols and references, every
   unresolved carrying a reason.
5. `reconcile` — the mutation cases below.

Only after each passes alone do we install, run a full scan, and verify the
emergent behaviour: progress emitted correctly, tasks queued and drained,
statuses advancing, and no state left in `discovered`.

## Fixture repo — the mutation cases need one, they cannot use this repo

`unparseable`, `rename`, `delete`, `modify` are not observable against a
real checkout; the test has to CAUSE them. So: a temporary git repo the test
builds, indexes, mutates, and re-indexes, asserting the delta.

    fixture: two files, x.rs calls z() in z.rs, one type, one field, one trait impl

| case | mutation | asserted |
|---|---|---|
| modify | change z()'s body | z's node updated; x's edge untouched; no churn elsewhere |
| delete symbol | remove z() | z's node DELETED; x's edge UNRESOLVED with target_name kept; NOT pointing at a file-less node |
| delete file | remove z.rs | file row gone, its nodes gone, x's edge unresolved |
| rename, module unchanged | move within the same module | file row path updated, id stable, NO node churn |
| rename, module changed | move to another module | declarations re-minted under the new module |
| unparseable | truncate x.rs mid-function | NOTHING removed; file marked with reason AND location; nodes dirty; x's edge intact |
| unparseable then fixed | restore x.rs | dirty cleared, file parsed, no residue |
| no-op | re-index unchanged | zero rows written, `modified_at` unmoved |

The last one is the one that silently regresses. Every other case is visible
when it breaks; a no-op that quietly rewrites 300 rows is invisible until
something else times out.

## Why fixtures rather than the live corpus

The live graph is useful for MEASURING (it produced every number in this design)
and useless for asserting: it changes under the test, it cannot be mutated
safely, and no case above can be caused on demand. Measure against the corpus;
assert against the fixture.
