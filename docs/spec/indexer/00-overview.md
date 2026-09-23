---
name: Indexer Flow
description: The one canonical indexing cycle — root scan to persisted edges — with the code that implements each stage
date: 2026-09-22
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

```
  ROOT SCANNER                                    TaskKind::ScanRoot
    scan_root.rs::find_git_roots(dir, exclusions) -> RootScan
    handlers/scan.rs::scan_root
        │  finds every git root under a watched directory
        ▼
  GIT REPO SCANNER                                TaskKind::ProcessGitFolder
    scan_repo.rs::scan_repo_files(repo_root) -> RepoScan
    structure.rs::plan_structure(..)          (pure: what to write)
    handlers/process.rs::process_git_folder
        │  PERSISTS FOLDER ENTRIES -> sensei.folders
        ▼
  MANIFEST SCANNER
    adapters/manifest::all_manifest_filenames / parse_manifest
    placement.rs::owning_manifest(file, repo_root, manifests)
    placement.rs::package_named_by(manifest, text)   -> the package NAME
    placement.rs::placement_of(file, package, root)  -> Placement { package, module }
    pipeline.rs::placement_on_disk(..)               (the IO half: climbs, reads)
        │  a file no manifest names has NO package, so it is NOT indexed —
        │  `Skipped::Unplaced`, a visible gap and never an invented package
        ▼
  FILE SCANNER                                    still ProcessGitFolder
    scan_logic.rs::plan_reindex(current, prior, hash) -> Plan { changed, touched, removed }
    handlers/process.rs, the barrier loop
        │  PERSISTS FILE ENTRIES -> sensei.files, at folders.rs::BARRIER_MTIME (0)
        │  ── STAGE 3 BARRIER (R14) ──
        │  every files row exists BEFORE any parse task for the folder runs,
        │  because node persistence FAILS CLOSED on a missing row (R13)
        ▼
        │  enqueues ONE TaskKind::ProcessFile per changed file
        ▼
  INDEXER, ONE FILE AT A TIME                     TaskKind::ProcessFile
    handlers/process.rs::process_file
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
           │           Without it every reference stays Reason::Unplaced and
           │           carries no fqn for an edge to point at.
           ▼
           persist.rs::write(store, folder_id, facts)
             1. NODES FIRST, collecting ids:
                  for each symbol -> indexer.rs::upsert_symbol(..) -> uuid
                  known: HashMap<fqn, uuid>          the id map
             2. EDGES, with those ids:
                  source_id = node_for(.., &mut known, ..)
                  target    = target_ref_of(&row.target)
                     Proven(fqn) + Origin::Local -> TargetRef::Internal {
                         on_miss: OnMiss::CreateStub }
                         a target no file has declared yet gets its node MINTED
                         and the edge carries THAT id
                     Proven(fqn) + Origin::Lib   -> TargetRef::Lib
                     Named(name)                 -> TargetRef::Unresolvable
                         no fqn at all, so nothing to point at
                  persist_edge_fact(folder_id, &fact, &known, ..)
             3. merge_edge_occurrences(edge_id, file_path, ..)
                  keyed BY FILE, so a re-scan replaces its own spans and
                  another file's survive
        │
        ▼  advance the fingerprint off BARRIER_MTIME — the handler's record
           that this file was processed. Without it plan_reindex re-enqueues
           the file for ever.
     CYCLE COMPLETE
```

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
external. Those unresolved references become linked as other files land, by the
upsert above.

---

## The incremental scan

A change arrives from the filesystem watcher rather than a full sweep. The
stages are the same; only the entry and the file SET differ.

```
  WATCHER                       watcher/root_watcher.rs
    a path changed under a watched root
        │
        ▼
  GIT REPO SCANNER, again       TaskKind::ProcessGitFolder
    handlers/process.rs::process_git_folder
        │
        ├─ current   = every visible file now on disk, with its mtime
        ├─ prior     = sensei.files for this folder, as (mtime, content_hash)
        └─ scan_logic.rs::plan_reindex(current, prior, hash_file)
             │
             │  TWO-TIER GATE, so an unchanged file is never read:
             │    1. mtime gate  — stat only. mtime equal to the stored one
             │                     => UNCHANGED. never read, never hashed
             │    2. hash gate   — mtime drifted, so hash it and compare
             │                     identical  => TOUCHED (refresh mtime only)
             │                     different  => CHANGED (reindex)
             ▼
        Plan { changed, touched, removed, unchanged, expected }
             │
             ├─ touched  -> upsert_scan_state(..) — refresh the mtime so the
             │              cheap gate hits next pass. NOT reindexed: the
             │              nodes and embeddings are still valid
             │
             ├─ unscannable -> upsert_scan_state_skipped(.., reason) and
             │              DROPPED from `changed`, so no doomed parse task is
             │              enqueued. Fingerprinting the skip is what stops an
             │              infinite re-index loop
             │
             ├─ removed  -> unresolve_edges_to_file, delete_nodes_by_file,
             │              delete_scan_state_file  (handlers/process.rs)
             │              plus scan::prune_vanished as a safety net for nodes
             │              that outlived their files row
             │
             └─ changed  -> STAGE 3 BARRIER (files row at BARRIER_MTIME)
                            -> one ProcessFile each
                            -> the same INDEXER stage as the full scan
```

`incremental.rs` classifies a detected change as OLD plus NEW, which is what
makes a rename distinguishable from a delete-plus-add:

| detected | files row | reparse |
|---|---|---|
| content changed, path same | touch `mtime` + `content_hash` | yes |
| path changed, content same | UPDATE the path, `id` unchanged | see R10.5 |
| path + content changed | update both | yes |
| added | create the row | yes |
| removed | delete the row, cascade | no — reconcile |
| touched, hash identical | refresh `mtime` only | no |

---

## Files are not nodes

`sensei.files` holds the files. `sensei.nodes` holds GRAPH nodes — symbols — and
each REFERENCES its file through `nodes.file_id`. A file is never a node.

So anything that wants file-level information reads `sensei.files`, and a view
wrapping nodes and edges joins to it for the file-level entries. `nodes.is_test`
and `nodes.is_exported` are per-SYMBOL columns, which is where a UI filters.

---

## Per-language state

| language | indexer | v1 parser |
|---|---|---|
| rust | v2 | deleted |
| typescript / javascript / svelte | v1 | present |
| python, java | v1 | present |
| sql, swift, kotlin, vue, c | v1 | present |

Each language migrates the same way: land its v2 adapter, prove it, delete
`languages/<that language>.rs`. There is **no fallback** — the two indexers mint
different identities (`languages/fqn.rs` against `indexer/fqn.rs`), so a graph
holding both could never join them. A file whose language has no v2 adapter yet
is served by v1; a file whose v2 adapter cannot place it is not indexed at all.

## Specs

`docs/spec/indexer/` — `01-scan-root`, `02-scan-repo`, `03-structure-write`,
`04-walk-rust`, `05-resolve`, `06-persist`, `07-reconcile`, `09-incremental`,
`10-cutover`, `11-file-index`. Whole-system rules (R1, R13, R14, S5, S7) are in
`docs/design/indexer.md`.
