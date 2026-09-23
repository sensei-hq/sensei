---
name: Indexer
description: The one canonical indexing cycle — root scan to persisted edges — with the code that implements each stage
date: 2026-09-23
status: current
---

# Indexer flow

One cycle, five stages, each persisting before the next begins. Every stage
below names the code that implements it, so this doc and the tree can be checked
against each other.

**The rule the whole shape rests on:** a stage never reaches for what it was not
given. The file scanner is TOLD its package, its module and what the scan knows;
it never climbs a directory and never queries for context. That is what makes
one file indexable alone, and it is enforced by
`lang::tests::no_adapter_reads_the_filesystem`.

---

## The full scan

Five task kinds, each a few lines that call parts proven on their own. Nothing
in this chain walks a tree twice or decides a rule a registry already owns.

```
  ROOT SCANNER                                    TaskKind::ScanRoot
    handlers/scan.rs::scan_root
    indexer/repo.rs::discover(root, exclusions)  one walk, `.git` only, DISK
    indexer/repo.rs::narrow(repos, changed)      event scope only
        │  PERSISTS REPOSITORY ROWS -> sensei.folders (kind='git')
        │  reconcile_roots ONLY when exhaustive && complete
        ▼
  REPO SCANNER                                    TaskKind::ProcessGitFolder
    handlers/repo_scan.rs::process_git_folder
    indexer/repo.rs::scan(repo, exclusions)      ONE walk, classified as it passes
    indexer/repo.rs::folder_tree(contents)       parents before children
        │  PERSISTS FOLDER ROWS -> sensei.folders
        │  PERSISTS FILE ROWS   -> sensei.files, at BARRIER_MTIME (0)
        │  ── STAGE 3 BARRIER ──
        │  every files row exists BEFORE any parse task, because node
        │  persistence FAILS CLOSED on a missing row (R13)
        ▼
        │  enqueues ONE ProcessManifest per manifest, then the GATE
        ▼
  MANIFEST PASS                                   TaskKind::ProcessManifest
    handlers/repo_scan.rs::process_manifest
    indexer/pipeline.rs::apply_manifest(pg, job)  THE per-manifest pass
        │  package name (placement) · dependencies · lockfile pins
        │  folder_commands · the module folder row
        │  a file no manifest names has NO package, so it is NOT indexed —
        │  a visible gap, never an invented package
        ▼
  THE GATE                                        TaskKind::ProcessRepoFiles
    handlers/repo_scan.rs::process_repo_files
        │  BLOCKED on every ProcessManifest of this repo, via the queue's
        │  ordinary dependency barrier — no counter of its own
        │  list_unparsed_files: skip_reason IS NULL AND parsed_at IS NULL
        ▼
        │  enqueues ONE ProcessFile per unparsed file
        ▼
  FILE INDEXER                                    TaskKind::ProcessFile
    handlers/process.rs::process_file
      lang::production_adapter_for_ext(ext)   the cutover frontier, ONE place
      -> pipeline.rs::placement_on_disk(file, repo, language)
      -> pipeline.rs::index_and_persist(pg, folder_id, FileInput)
           │
           ├─ index.rs::index_file(FileInput { repo, path, mode,
           │                                   package, module, text, world })
           │    ├─ lang::adapter_for_path(path)        which adapter reads this
           │    ├─ adapter.read(&source, &TypeHomes::unknown())
           │    │      the WALK. One parse (R1). No cross-file table (S5).
           │    │      -> FileFacts { symbols, references, relations, imports }
           │    └─ resolve.rs::resolve(facts, grammar, world)
           │           the LADDER. Turns each Observation::Named(fqn) the walk
           │           minted into Resolution::Resolved { fqn, via }.
           ▼
           persist.rs::write(store, folder_id, facts)
             1. NODES FIRST, collecting ids:
                  for each symbol -> indexer.rs::upsert_symbol(..) -> uuid
                  known: HashMap<fqn, uuid>          the id map
             2. EDGES, with those ids:
                  Proven(fqn) + Origin::Local -> TargetRef::Internal {
                      on_miss: OnMiss::CreateStub }
                      a target no file has declared yet gets its node MINTED
                      and the edge carries THAT id
                  Proven(fqn) + Origin::Lib   -> TargetRef::Lib
                  Named(name)                 -> TargetRef::Unresolvable
             3. merge_edge_occurrences(edge_id, file_path, ..)
                  keyed BY FILE, so a re-scan replaces its own spans
        │
        ▼  advance the fingerprint off BARRIER_MTIME
     CYCLE COMPLETE -> EmbedNodes -> DetectCommunities (folder -> `indexed`)
```

**Triggers, scope and the scenarios for every entry point** are in
`15-triggers.md`, with executable scenarios in `16-scenarios.md`. The short
version: **eleven** production conditions start a cycle — ten raise
`Scope::Full` and one (a watcher batch) raises `Scope::Events` — and two more
(an exclusion added, a root removed) raise no task at all, because a deletion
needs no walk.

### Why no separate heal pass

The fqn is the join key, so an edge never needs relinking:

1. file A references `p::b::Widget`, which nothing has declared yet
2. `OnMiss::CreateStub` mints a node for that exact fqn and the edge stores its id
3. later, file B — which DECLARES `Widget` — is indexed; `upsert_symbol` upserts
   the SAME fqn and fills the row in
4. the edge already points at it

Healing is the upsert. There is no `target_id` fix-up step, no pending queue,
and no relink pass.

### Why an empty world is safe

`resolve::World` has five fields and four are repo-wide artifacts of a completed
pass. A single file supplies only `first_party` (`pipeline::TellFile`). Each of
the other four documents the same contract:

> Empty means "not supplied", and then nothing is reclassified — the previous
> behaviour, and never a guess.

and the rung that would call a member external guards on
`!first_party_members.is_empty()`. So a file indexed alone resolves what its own
text establishes (S7) and leaves the rest unresolved — never mis-filed as
external.

---

## The stages

One section per stage in the graph above.

### ROOT SCANNER — `scan_root`

*Which repositories exist under a watched directory.*

- **Resolves the EFFECTIVE watch root first.** A scan aimed at a path already
  inside a watch root indexes under that root; only a path under no existing
  root becomes a new `folders_to_watch` row.
- **Reads that root's exclusions from the DB.** `root_exclusion_prefixes`
  resolves each entry — written relative to the root — to an absolute prefix.
  FAILS CLOSED: a DB error propagates rather than degrading to an empty list,
  because "no exclusions" and "could not read them" are different facts, and
  collapsing them indexes exactly what the user excluded.
- **An exclusion is a subtree prefix at any depth.** With `~/Work` as the root,
  `group-a` excludes `~/Work/group-a`, and `group-a/sub` excludes
  `~/Work/group-a/sub`. `scan_logic::is_excluded` is the ONE owner of that rule
  — the watcher calls it rather than keeping a copy — and it matches
  segment-anchored, so `Code` never excludes `Coder`.
- **Discovers from DISK, never from the database.** A repository is a directory
  holding a `.git`, which is a DIRECTORY for a clone and a FILE for a submodule
  or linked worktree; matching only directories misses every checked-out
  submodule. Discovery therefore cannot be wrong because the DB is stale, and a
  repository created while the daemon runs is found by the walk rather than
  missed by a lookup.
- **Narrows by scope.** An event batch keeps only the repositories that OWN a
  changed path (`repo::narrow`, longest prefix, shared with the watch-root
  resolver so the two can never disagree).
- **Reconciles only when it saw everything** — stale / moved / vanished roots,
  gated on `scope.is_exhaustive() && discovered.is_complete()`.
- **Hands each repository its slice** (`scope.under(repo)`), and the SAME
  exclusion list goes to `RootWatcher::register`.

### REPO SCANNER — `process_git_folder`

*Everything structural inside ONE repository.*

- **Takes a scope and never widens it.**
- **Walks the repo ONCE, classifying as it passes** — files, folders, manifests,
  lockfiles and unsupported files out of one traversal. A separate manifest glob
  would re-walk the tree AND need its own exclusion rules, a second place for
  them to drift.
- **Honours `.gitignore` here, and only here.** Inside a repository its own rules
  are the right answer to "does this belong in the index". When they were not
  honoured, generated files the walk could not see landed in `removed`, had
  their nodes deleted, were re-created by the next build, and churned for ever.
- **Writes the folder tree parents-first**, derived from the ANCESTORS of every
  entry rather than only the directories the walk yielded: a `.gitignore` that
  excludes a directory's contents but re-includes one file yields the FILE
  without the DIRECTORY, and a file row pointing at a folder nobody wrote fails
  closed on insert.
- **Writes a `files` row for every file examined**, indexed or not — the stage 3
  barrier for source, and the stuck-skip record for everything else.
- **Separates observed deletions from inferred ones**
  (`docs/spec/indexer/15-triggers.md` §2 S4).

### MANIFEST PASS — `process_manifest`

*One task per manifest, all of them before any file is parsed.*

- **Asks the adapter for everything it knows** — package name, dependencies,
  lockfile pins, named commands — rather than taking `.name` and re-opening the
  file elsewhere for the rest. `apply_manifest` is the ONE per-manifest pass.
- **Pairs each manifest with its nearest READABLE lockfile.** A manifest states
  a range and serves its own directory; a lockfile states what is installed and
  serves a whole SUBTREE. `tools/session-report` has its own `Cargo.lock` and it
  genuinely disagrees with the root's.
- **Records commands per MANIFEST, not per repo root.** The previous pass read
  only `<repo>/<manifest>`, so every member package of a monorepo had its
  scripts silently undiscovered.
- **Names no package rather than inventing one.** A manifest may declare only a
  workspace; a fabricated name gives every file under it an fqn nothing joins.

### THE GATE — `process_repo_files`

*No file is parsed until every manifest has been read.*

- **Blocked on every `ProcessManifest` of its repo**, using the queue's ordinary
  dependency barrier. It owns no counter: `complete` and `fail` both strike the
  dep and promote the task when the list empties. A second implementation would
  disagree with that one the first time a manifest failed.
- **Fails OPEN.** A broken manifest still releases it: files whose placement came
  from that manifest are skipped as unplaced — a visible gap — where holding it
  shut would strand every OTHER file in the repository behind one bad file.
- **Carries no work list.** It reads the unparsed set back from `files`
  (`skip_reason IS NULL AND parsed_at IS NULL`, the lifecycle's DISCOVERED
  state), which makes a re-run idempotent: anything already parsed has
  `parsed_at` set and is not re-enqueued.

### FILE INDEXER — `process_file`

*One file in, nodes and edges out.*

- **Asks the registry which languages it owns.** `lang::PRODUCTION_LANGUAGES` is
  the cutover frontier and the ONE place it is written; the handler dispatches
  on the answer rather than deciding it.
- **Carries the language to placement.** `placement_on_disk`'s module-path rule
  is the adapter's own — rust drops a trailing `mod`/`lib`, javascript KEEPS a
  trailing `index` — so a hardcoded language mints the wrong fqn for every
  symbol in the file.
- **A file it cannot PLACE is not indexed.** No fallback: `languages/fqn.rs` and
  `indexer/fqn.rs` mint different identities, so a graph holding both could
  never join them.

## Files are not nodes

`sensei.files` holds the files. `sensei.nodes` holds GRAPH nodes — symbols — and
each REFERENCES its file through `nodes.file_id`. A file is never a node.

So anything that wants file-level information reads `sensei.files`, and a view
wrapping nodes and edges joins to it for the file-level entries. `nodes.is_test`
and `nodes.is_exported` are per-SYMBOL columns, which is where a UI filters.

---

## Per-language state

`lang::PRODUCTION_LANGUAGES` is the frontier, in one place. A language joins it
only together with deleting `languages/<that language>.rs` — no fallback,
because the two mint different identities.

| language | adapter | this indexer produces | legacy parser |
|---|---|---|---|
| rust | yes | **yes** | **unreachable** — still registered for DETECTION |
| typescript / javascript | yes | not yet | live |
| svelte | yes | not yet | live |
| java, python | yes | not yet | live |
| sql, swift, kotlin, vue, c | **no** | no | live |

"Unreachable" is not "deleted", and the difference matters: `languages/mod.rs`
still registers `rust_lang::RustAdapter` because the registry also answers
"what language is this file?", and `languages/mod.rs:1007` asserts `.rs` still
resolves to it. Its PARSE half has no caller; splitting detection from parsing
is what would let the file go.

Measured 2026-09-23 over the live graph: rust is 33,716 of 467,595 nodes. The
legacy SCAN and PROCESS orchestration is GONE — `process_git_folder`,
`indexer/scan_root.rs`, `plan_reindex`, `classify_folders`, `find_git_folders`,
`all_directories`, `count_indexable_files`, the `standalone` folder kind and
`LOCKFILE_NAMES` were all deleted. What remains under `crate::languages` is
parsers, one per language, retiring as each is flipped.

## The rest of this folder

Start here, then follow the leg you need. Every file carries a `status:` in its
frontmatter so supersession is visible without reading it.

**Reference — read these before any number or letter makes sense**

| | |
|---|---|
| [`17-vocabulary.md`](17-vocabulary.md) | what a NODE, an EDGE and a `via` are; what every measured number counts, in what unit, over what population; ceiling vs ratchet |
| [`18-acceptance.md`](18-acceptance.md) | what `A1`–`A9` mean, which test enforces each, what bound it holds |

**The flow, leg by leg**

| | |
|---|---|
| [`02-scan-root.md`](02-scan-root.md) | which repositories exist *(superseded — see this file and `15-triggers.md`)* |
| [`03-scan-repo.md`](03-scan-repo.md) | everything structural inside one repository |
| [`04-library-discovery.md`](04-library-discovery.md) | third-party libraries |
| [`05-structure-write.md`](05-structure-write.md) | folder and file rows *(superseded in part)* |
| [`06-walk-rust.md`](06-walk-rust.md) | the rust walk |
| [`07-walk-js.md`](07-walk-js.md) | the javascript/typescript walk |
| [`08-resolve.md`](08-resolve.md) | the ladder that places a reference |
| [`09-persist.md`](09-persist.md) | nodes first, then edges |
| [`10-reconcile.md`](10-reconcile.md) | what a re-scan removes |
| [`14-file-index.md`](14-file-index.md) | one file in, nodes and edges out |

**Entry points and behaviour**

| | |
|---|---|
| [`15-triggers.md`](15-triggers.md) | every production condition that starts a cycle, and the scope it starts with |
| [`16-scenarios.md`](16-scenarios.md) | Gherkin for every flow, each naming the test that pins it |
| [`11-progress.md`](11-progress.md) | what a scan reports while it runs |
| [`12-incremental.md`](12-incremental.md) | *(superseded — its S2 is inverted; discovery reads the FILESYSTEM)* |

**History**

| | |
|---|---|
| [`01-files-entity.md`](01-files-entity.md) | the completed `scan_state` → `files` migration |
| [`13-cutover.md`](13-cutover.md) | *(superseded for the scan/process half, which is done)* |

Whole-system rules (R1, R13, R14, S5, S7) and the acceptance criteria
themselves are in [`docs/design/indexer.md`](../../design/indexer.md).
