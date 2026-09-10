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

**`docs/plans/indexer-v2-rust.md` is SUPERSEDED by this file plus the ten
specs.** It deferred all DDL to its "step 9", which contradicted stage 0 below;
that conflict is resolved in favour of stage 0 (whole-system spec §7h, D10).
Keep it only as the historical record of how the Rust steps were first drafted.

| # | spec | builds | depends on |
|---|---|---|---|
| 0 | `spec/indexer/00-files-entity.md` | **ALL DDL**: `scan_state`->`files` + `id` + `nodes.file_id`, parse-detail column, node_kind/edge_kind widening, occurrences gin index, `project_commands`->`folder_commands`, ORPHANED sweep | — (dbd `reconcile`; pre-release, no `migrations/`) |
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

### Why stage 0 takes ALL the DDL, not just the files migration

Deferring the schema bought three workarounds for constraints stage 0 removes:
the `parameter` kind placeholder (already in committed code, with twelve
comments citing a build step that no longer exists), and R10.1's two-disjunct
edge predicate. Both are now WITHDRAWN in the spec. `props.claims` stays in
jsonb — that one was a deliberate choice, never a deferral. Writing a
workaround for a constraint you are about to remove is waste, and the
workaround outlives the constraint.

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

### The manifest pass, concretely

    the file walk (already visiting every file)
      -> for each file, ManifestAdapter::accepts(filename)?
           -> dispatch to that adapter
           -> store its facts on the FOLDER CONTAINING the manifest

**Do NOT glob a second time.** `scan_repo` already walks every file for the file
set. Testing each against `accepts()` as it passes costs one string comparison
per file and needs no extra traversal. A separate manifest glob would re-walk
48,654 files to find perhaps 200 — and would then need its own exclusion rules,
which is a second place for them to drift.

**Do NOT hardcode the filename list.** Each adapter declares
`manifest_filenames()` / `manifest_extensions()`, and the dispatch table is the
registry. The glob set is DERIVED from the adapters, so adding an ecosystem is
one impl and no edit anywhere else. That is the whole reason the trait exists —
the doc comment names the ten hardcoded sites it replaced.

**Folder level is the right home**, and it is already the shape:
`project_commands.folder_id`. A manifest sits AT a directory and describes that
directory's subtree, so its facts belong to that folder:

| fact | from | example here |
|---|---|---|
| commands | `parse_commands` | `app/` gets `dev`, `build`, `test:unit`, `test:e2e` |
| dependencies | `parse_dependencies` | with versions, per manifest |
| hierarchy | `detect_workspace_members`, `is_workspace_root` | root `Cargo.toml` -> six crates |
| language / framework | `stack_labels` | |
| role | `infer_role` | |

This repo has SIX manifest-bearing folders (root `Cargo.toml` + `Makefile`, and
`package.json` in `app/`, `dojo/`, `website/`, `marketplace/`), so folder-level
storage is not a refinement — a repo-level store would collapse four different
`build` commands into one and lose which directory each runs in.

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

## Repositories, projects and commands — the naming, settled

Checked against the live schema rather than inferred:

| entity | table | rows | what it is |
|---|---|---:|---|
| repository | `sensei.repositories` | 68 | the IDENTITY, keyed on `repo_key` (normalized remote). Survives a re-clone, rename or move. No `project_id` — the schema permits a repo in several projects. |
| its placement | `sensei.folders` (69 carry `repository_id`, 67 distinct repos) | 9,378 | where it sits TODAY. Only the repo-root folder carries the link; subfolders resolve by nearest ancestor. |
| project | `sensei.projects` | 147, of which **66 group a repository** | the user-viewable GROUPING of repositories. |

**The expected cardinality is FEWER projects than repositories**, and the live
data agrees once the junk is excluded: **66 projects over 67 repositories** —
63 hold one repo, 3 hold two, and **no repo belongs to more than one project**
(measured max = 1). The schema permits M:N; the data is 1:N.

The 147 is inflated by **81 projects that group ZERO repositories** plus 2 with
no folders at all. They were minted per-folder from directories the old scanner
classified as "non-git repos" — `find-me-board` (1,230 folders),
`pljava-1_6_7`, and `ms-dotnettools.csdevkit`, a VS Code extension cache under
a transcripts directory. Under stage 1 those folders stop being roots, so these
projects must be resolved rather than silently left behind. See 01-scan-root S6.

So `scan_root` produces BOTH — a repo-root folder row and a `repositories`
row. Two clones of one remote are two folders and one repository. This was
already the design and it is correct.

`project_commands` is renamed **`folder_commands`** in stage 0 (D11). It is
keyed on `folder_id`, has no project reference, and its own comment says "per
folder". `repository_commands` would be wrong too: measured, 572 commands span
**58 folders but only 36 repositories**, so a repo-grained key collapses 22
folders' command sets and loses which directory each `build` runs in.
`project_libraries` (keyed `project_id`) and `project_dependencies`
(project->project) keep their names — correctly named already.

## Status

| | |
|---|---|
| whole-system spec | written and REVIEWED — `docs/design/indexer-v2.md` |
| consistency review | run, 38 findings, folded in at `f9393c52` |
| per-stage specs | **WRITTEN** — `docs/spec/indexer/00..10` |
| superseded | `docs/plans/indexer-v2-rust.md` (DDL ordering; historical only) |
| code | `bc343622`, nothing since |

The review found ten CRITICAL items, all of them stale text that would have
produced wrong code — five places still prescribing demotion after it had been
rejected, and an R10.8 that required inbound edges unresolved while every FK
cascades. Those are fixed in the whole-system spec, and the ten specs carry the
corrected version rather than the draft.

Three measured corrections came out of it, none from reading: COMPLETE is
346,506 not 354,653 (the larger number is COMPLETE + ORPHANED); the three file
counts were one experiment at three sizes; and R11.2's upstream half is
unbuildable today because exactly 2 of 1,121 library rows carry any URL.

## Known-wrong in the committed code — the three stage specs that fix them

`bc343622` names these in its own commit message and they are not yet fixed:

| defect | fixed by |
|---|---|
| `demote_v2_symbol` — keeps a node and nulls `file_path`, so `target_id` stays non-null and every consumer reads it as resolved (67,839 such edges measured) | `07-reconcile.md` S3–S6 |
| `v2_edges_contributed_by`'s narrowing `AND (s.fqn = ANY($3) OR s.resolved = false)` — an edge whose source this file deleted, resolving elsewhere, is never revisited | `07-reconcile.md` S7 |
| 11 self-verifying column mappings, 6 in `v2_symbol_unchanged` — the round trip reads back props the same writer wrote | `06-persist.md`, last verification row |
