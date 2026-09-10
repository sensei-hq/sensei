# Stage 3 — structure write: the barrier

Whole-system spec: `docs/design/indexer-v2.md` R14, R13 (the walk creates file
rows), R10.7b (folder rollup). Depends on stage 2.

## 1. Purpose

Write every folder row and every file row for a repo, then — and only then —
enqueue one parse task per file. The gap between "all structure exists" and
"work begins" is the barrier, and three separate problems do not arise because
of it.

Serves G2: a repo's shape is visible and its progress denominator is known
before any parsing happens, so "how far along is this" is answerable from the
first second rather than at the end.

## 2. Inputs and outputs

    plan_structure(FileSet, existing: &StructureSnapshot) -> StructurePlan   PURE
    apply_structure(StructurePlan)                        -> StructureIds    IO
    enqueue_parse_tasks(StructureIds)                     -> usize           IO

`plan_structure` is pure and is where all the decisions live — what is new,
what moved, what vanished. `apply_structure` executes a plan it does not think
about. That split is what makes the barrier testable without a database.

## 3. Requirements

- **S1** (R14). Create ALL folder rows and ALL file rows before enqueuing ANY
  parse task. Not "mostly before" — the barrier is a point in time, and a
  single task enqueued early reintroduces every problem it removes.
- **S2** (R13). File rows are created by the WALK. Node persistence LOOKS UP
  `file_id` and FAILS CLOSED if absent — it must NOT get-or-create. A
  get-or-create would mint a phantom `files` row with no `mtime`, no
  `content_hash` and no `indexed_at`: the 8,147-ORPHANED problem one table
  over, and worse, because the foreign key now certifies it.
- **S3** (R14, R10.7b). At the barrier the complete post-filter file set is
  known. Write it to `folders.props.expected_files` — the denominator
  `sensei.folder_completeness` already divides by. Free at this instant,
  a second full count anywhere else.
- **S4** (R13). Every file row starts at lifecycle `discovered`. A file row
  with no successor state is a parse task that never ran, and that must be
  distinguishable from a file that does not exist.
- **S5** (R10.7b). Folder status rolls up through `sensei.folder_completeness`,
  the recursive view that already exists. Extend it to carry unparseable
  counts; do not add a second rollup mechanism, and do not write a status
  column that a trigger has to maintain.

## 4. Failure modes

| input | this stage does |
|---|---|
| the file set is empty | write the folder rows, write `expected_files = 0`, enqueue nothing. A repo of only ignored files is a real state. |
| a folder row insert fails | abort the whole plan. A partial structure means the barrier never happened, and half a denominator is worse than none. |
| a file exists in the DB but not in the new set | it is REMOVED — hand it to reconcile (R10.8), do not delete the row and let the cascade run. |
| a file's parent folder is missing from the plan | a bug in `plan_structure`. Fail loudly; do not create the folder on the fly, which would hide the ordering defect. |
| enqueue fails after structure is written | the structure is still correct and the files sit at `discovered`. Report the count that failed to enqueue; a re-run is safe because S1's writes are idempotent. |

## 5. Checkpoint output

    {"stage":"03-structure-write","at":"<iso8601>","repo":"…","repo_id":"…",
     "folders_created":<n>,"folders_updated":<n>,"folders_removed":<n>,
     "files_created":<n>,"files_updated":<n>,"files_removed":<n>,
     "expected_files":48665,
     "barrier_at":"<iso8601>","tasks_enqueued":48646,
     "files_at_discovered":48646,
     "sample_file":{"path":"crates/senseid/src/indexer/fqn.rs","file_id":"…",
                    "folder_id":"…","lifecycle":"discovered",
                    "content_hash":"…","enqueued":true}}

`tasks_enqueued` should equal the INDEXED file count (48,646), not the total
row count (48,665) — skipped files get rows but no parse task. If those two
numbers are equal, the skip filter is not being applied at enqueue.

`barrier_at` must be earlier than every parse task's start. That is checkable
after the fact and is the only direct evidence S1 held.

## 6. Verification

| test | mutation that must break it |
|---|---|
| `plan_structure` classifies added / updated / removed / unchanged from two snapshots, with no database | give it a `&PgPool` |
| no parse task exists before the last file row is written | move the enqueue inside the file-write loop |
| persisting a node whose `file_id` is absent returns an ERROR | add a get-or-create fallback |
| `expected_files` equals the file count at the barrier | compute it with a second query later |
| a removed file goes to reconcile, not to `DELETE` | replace the reconcile call with a delete |
| every new file row is at `discovered` | default the lifecycle to `parsed` |
| `folder_completeness` returns the new counts with no new column on `folders` | add a status column |
| re-running the whole stage changes no row | make any write non-idempotent |

The get-or-create test is the load-bearing one. It is the single easiest
"convenience" to add during stage 6 and it defeats the entire purpose of
stage 0's foreign key.

## 7. Watch out

**Four things follow from the barrier, and three of them are problems that
simply do not arise** — this is the reasoning to preserve if the ordering is
ever questioned:

1. **No race.** Parse tasks run concurrently. If each did get-or-create, two
   tasks touching one file would race to insert it. Creating the rows upfront,
   single-threaded, removes the race rather than locking around it.
2. **The denominator is free** (S3).
3. **A stalled parse is visible** (S4). Today it is indistinguishable from a
   file that does not exist.
4. **Order independence holds** (R6). No parse task creates shared state; each
   fills in its own row and writes its own nodes.

**Do not "optimise" by enqueuing as you walk.** It looks like a latency win and
it costs all four properties. The walk is fast; parsing is what takes the ~20
minutes.

**`clear_scan_state_for_root` is how the ORPHANED population was created** —
every forced reindex calls it, dropping file rows without touching nodes. After
stage 0 the FK makes that impossible, but the CALLER still exists. Find it and
make it go through reconcile, or the next forced reindex fails loudly on the
foreign key. Loud is better than the current silence, but neither is done.

## 8. Definition of done

- `plan_structure` is pure and unit-tested on snapshot pairs covering add,
  update, remove, rename and unchanged.
- The barrier is provable after the fact: `barrier_at` precedes every task
  start in the run.
- `expected_files` is populated and `folder_completeness` reads correctly
  against it.
- Node persistence fails closed on a missing `file_id`, verified by a test.
- `clear_scan_state_for_root`'s callers are resolved, not left to fail later.
- Stages 1–3 together now run end to end with NO parsing and no language code.
