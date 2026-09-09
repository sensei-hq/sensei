# Indexer v2 — execution sequence and spec index

The master plan. Each stage has its OWN spec, implementable and verifiable on
its own. This file is the order and the dependency between them; it holds no
design detail of its own.

Source of the split: `docs/design/indexer-v2.md` (R1..R15, A1..A8, D1..D9) is the
whole-system spec. The per-stage specs below carve it into buildable pieces and
must not contradict it — where they do, the whole-system spec wins and the
per-stage one is a bug.

## Sequence

Each row is startable only when everything above it is done and verified. This
ordering is not preference: each stage consumes a structure the previous one
creates.

| # | spec | builds | depends on |
|---|---|---|---|
| 0 | `spec/indexer/00-files-entity.md` | rename `scan_state`->`files`, add `id`, `nodes.file_id` | — (DDL, via dbd) |
| 1 | `spec/indexer/01-scan-root.md` | find repo roots, apply root exclusions | 0 |
| 2 | `spec/indexer/02-scan-repo.md` | submodules, subtrees, file/folder discovery, gitignore, **dependency manifests, commands** | 1 |
| 2b | `spec/indexer/02b-library-discovery.md` | packages -> libraries, skills, agents, docs corpus | 2 |
| 3 | `spec/indexer/03-structure-write.md` | folder + file rows, the barrier, `expected_files` | 2 |
| 4 | `spec/indexer/04-walk-rust.md` | one parse -> `FileFacts` (symbols, refs, relations) | 3 |
| 5 | `spec/indexer/05-resolve.md` | the shared ladder, reason codes, reach identity | 4 |
| 6 | `spec/indexer/06-persist.md` | `FileFacts` -> rows, lossless at the boundary | 5 |
| 7 | `spec/indexer/07-reconcile.md` | claim diff, dirty vs delete, cascade ordering | 6 |
| 8 | `spec/indexer/08-progress.md` | per-stage checkpoints, the tailable file | 3 (usable from there) |
| 9 | `spec/indexer/09-incremental.md` | changed files -> repos, the inverse traversal | 7 |
| 10 | `spec/indexer/10-cutover.md` | differential harness, switch rust only, retire v1 | 9 |

Steps 1-3 can be verified without any parsing at all. Step 4 needs no database.
Only 6 onward touch persistence. That is deliberate — it front-loads everything
cheap to test.

## Manifests and commands: ALREADY BUILT — correcting an earlier draft of this plan

An earlier version of this section specified dependency and command extraction as
work to be done. It is not. `ManifestAdapter` exists as a deliberate sibling to
`LanguageAdapter` (`crates/senseid/src/adapters.rs`), created to replace "ten
spaghetti sites" of hardcoded `if path == "package.json"` chains, and it is
wired and producing data.

The trait already provides everything this pipeline needs:

    manifest_filenames / accepts / ecosystem
    parse_dependencies        -> Vec<DepVersion>
    detect_workspace_members  -> Vec<PackageInfo>
    parse_commands            -> Vec<DiscoveredCommand>
    is_workspace_root / stack_labels / infer_role

TEN ecosystems implement it: cargo, npm, pyproject, go, maven, gradle, ruby,
dotnet, composer, swiftpm.

It is CALLED — `manifest_adapter_for_filename` from `libraries/registry.rs`,
`indexer/lib_indexer.rs` and `scan_logic.rs`; `parse_commands` from
`tasks/handlers/libraries.rs:575` — and it PERSISTS to
`sensei.project_commands (id, folder_id, raw_name, command_line, category,
ecosystem, source_file, discovered_at)`.

Live: **572 commands across 58 folders and 4 ecosystems.** `source_file` already
records the declaring manifest, so the per-manifest scoping this plan called for
is present — the working directory is its dirname.

### So what v2 actually changes here

Not the extraction. Only WHERE IT RUNS. Today it happens in a `libraries` task
handler, separate from the scan. Under R14 it belongs in `scan_repo`, before the
structure barrier, because:

- the same walk that finds files finds the manifests, so reading them twice is
  waste
- workspace members define package boundaries, which stage 2b needs BEFORE any
  parsing (see below)
- a repo's commands should be known as soon as the repo is known, not after a
  separate handler happens to run

### The one genuine gap

`category` is unpopulated on some rows (`coverage`, `quality:gate` have none;
`test:system` and `install` do). A command with no category cannot be answered
against "how do I test this", which is the G1 question the data exists to serve.
Worth a look, but it is a field to fill, not a subsystem to build.

## Library discovery is its own stage, and it does NOT depend on parsing

`library_packages` is keyed by package NAME, not by node id. So the grouping can
be populated as soon as `scan_repo` has read the manifests — before any file is
parsed, and independently of whether parsing ever succeeds.

Stage 2b therefore:
- takes the package set from the manifests
- reads `sensei.library.json` from a dependency when one is present (R10.7h),
  which brings the library, its skills, its agents and its docs corpus in one go
- records PROVENANCE on every grouping — declared-by-dependency,
  declared-by-user, or absent (R11.1)
- leaves ungrouped packages ungrouped, which is complete rather than wrong

It closes the chain that is currently one link short (R10.7g):
`node -> package -> library -> pages/skills/agents`. Measured: 130 pages across
128 components, 10 skills and 6 agents are already indexed and unreachable from
any node, because `library_packages` has 0 rows.

Placed at 2b rather than late BECAUSE it is independent — putting it after
persistence would imply a dependency that does not exist, and would delay the
single largest G1 payoff behind the slowest part of the pipeline.

## What every per-stage spec must contain

Uniform, so an implementer never has to guess where something is:

1. **Purpose** — one paragraph, and which of G1/G2 it serves.
2. **Inputs and outputs** — exact types. Which are PURE and which do IO.
3. **Requirements** — numbered LOCALLY (S1, S2 …), each independently testable,
   each citing the whole-system requirement it derives from (R-something).
4. **Failure modes** — what this stage does when its input is wrong, missing or
   malformed. Never "cannot happen".
5. **Checkpoint output** — the JSON line it appends, and the SAMPLE it includes
   so a reader can eyeball the shape rather than trust a count.
6. **Verification** — the tests that prove it, and for each, the one-line
   mutation that must break it.
7. **Watch out** — the specific trap, with the measurement behind it.
8. **Definition of done** — what must be true before the next stage starts.

A spec missing 4, 6 or 8 is not ready to implement. Those are the three that get
skipped and the three that cost the most later.

## Fixtures

`09-incremental` and `07-reconcile` need the mutation fixture (a temp git repo
the test builds and mutates). It is specified once, in `07-reconcile`, and
referenced by `09`. Not duplicated.

## Status

| | |
|---|---|
| whole-system spec | written — `docs/design/indexer-v2.md` |
| per-stage specs | NOT YET EXTRACTED |
| consistency review | run at checkpoint; findings fold into the extraction |
| code | `bc343622`, nothing since |

The extraction is mechanical but not trivial: several sections of the
whole-system spec were superseded mid-session, so the split must carry the
CORRECTED version. The review exists to catch exactly that before it is copied
into ten files instead of one.
