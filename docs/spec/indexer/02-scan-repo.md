# Stage 2 — scan repo: submodules, subtrees, manifests, files

Whole-system spec: `docs/design/indexer-v2.md` §7g (R15), R14, R10.7c, R10.7h.
Depends on stage 1.

## 1. Purpose

Given ONE repo root, discover everything structural inside it: the submodules
it declares, its dependency manifests and the commands they expose, and its
complete post-filter file and folder set. It produces structure; it parses no
source.

Serves G1 directly through commands and dependencies ("how do I test this",
"what does this use") and G2 through the file/folder shape of the repo.

## 2. Inputs and outputs

    find_submodules(gitmodules_text: &str)        -> Vec<SubmoduleDecl>   PURE
    find_subtrees(log: &GitLogFacts)              -> Vec<SubtreeGuess>    PURE
    read_manifests(files: &[PathBuf])             -> Vec<ManifestFacts>   PURE per file
    find_files(root, filters)                     -> FileSet              fs read
    save_files_and_folders(FileSet)               -> ()                   IO

Four of the five are PURE and take text or paths, not a repo. `find_submodules`
takes the CONTENT of `.gitmodules`, not its path, so its whole test suite is
string literals.

## 3. Requirements

- **S1** (R15). Read `.gitmodules` at the repo root. Declared submodules are a
  working-tree fact, not an inference.
- **S2** (R15). **A submodule IS a repo root: enqueue a `scan_repo` for each.**
  This is why submodule discovery lives here and not in scan root — a
  `.gitmodules` file is INSIDE a repo, and scan root does not open repo
  contents. The root set therefore grows during the scan-repo wave. The
  incremental path (stage 9) reads the complete root set from the DATABASE, not
  from scan root's output, which is what makes longest-prefix matching correct
  for a path under a submodule.
- **S3** (R15). Subtree detection is BEST-EFFORT enrichment with provenance
  `inferred_from_history`, never a structural fact anything depends on. Record
  it; do not let a folder's identity rest on it.
- **S4.** Use `build_walker` (`tasks/handlers/helpers.rs:118`) for file
  discovery. **It already does what this stage needs** — `.gitignore`,
  `.ignore`, global gitignore, `.git/info/exclude`, and `require_git(false)` so
  ignore files apply in a non-git directory too. Do not write a second walker;
  the fs-watcher and the scan sharing one is what stopped a documented
  add/prune churn loop.
- **S5.** Test every file against `ManifestAdapter::accepts()` AS IT PASSES in
  the walk of S4. Do not glob a second time (see §7).
- **S6.** Store manifest facts at the FOLDER containing the manifest —
  commands, dependencies, workspace members, stack labels, role. This is
  already the shape (`folder_commands.folder_id` after stage 0's D11 rename).
- **S6b.** **Extend `ManifestAdapter` to LOCKFILES**, because the manifest
  gives a range and only the lockfile gives the pin (02b S11). Two new trait
  methods, defaulted so no existing adapter breaks:

      fn lockfile_filenames(&self) -> &[&'static str] { &[] }
      fn parse_lockfile(&self, content: &str) -> Vec<PinnedVersion> { vec![] }

  Same shape as the rest of the trait — `content: &str`, pure, unit-testable
  on literals. `accepts()` must keep answering for MANIFESTS only; a lockfile
  is a distinct file class and routing one into `parse_dependencies` would
  parse the wrong grammar. Add `accepts_lockfile()` alongside it.

  The glob set stays REGISTRY-DERIVED (S5): the walk tests each file against
  both predicates as it passes. Still one traversal.

- **S6c.** **Resolution: NEAREST LOCKFILE AT OR ABOVE the manifest's folder,
  stopping at the repo root.** Not "same folder", not "repo root" — measured
  here, all three are different:

  | lockfile | serves |
  |---|---:|
  | `/Cargo.lock` | the root `Cargo.toml` + **8 workspace members** under `crates/` = 9 manifests |
  | `app/src-tauri/Cargo.lock` | 1 — and it must win over the root's, or that crate gets another workspace's pins |
  | `tools/session-report/Cargo.lock` | 1, same reason |
  | `app`, `website`, `dojo` `bun.lock` | 1 each, sibling to their `package.json` |
  | `marketplace/package.json` | **NONE.** No lockfile anywhere above it. |

  `crates/senseid/Cargo.toml` has no sibling lock and walks up to `/Cargo.lock`.
  `app/src-tauri/Cargo.toml` finds its own FIRST and must never reach the
  root's. A "use the repo root's lockfile" rule gets two of the three wrong.

  This resolution is ONE function, not per-adapter — the walk-up is identical
  for every ecosystem; only the filenames differ, and those come from
  `lockfile_filenames()`.

- **S6d.** **No lockfile is a real state, and the version stays a RANGE.**
  `marketplace/package.json` is the live example. Record the range with its
  operator intact (`^1.2.3`), never the floor — `clean_version` strips the
  operator and produces `1.2.3`, which is indistinguishable from a pin and is
  the R4 fabrication 02b S11 documents. A range recorded as a range can be
  resolved later; a range recorded as a pin will be believed.
- **S7** (R14). Create ALL folder and file rows, then stop. Enqueuing parse
  tasks is stage 3's barrier, not this stage's.
- **S8** (02b S11). `find_libraries` extends the same manifest pass: for each
  `DepVersion` that is NOT a local sibling, enqueue `(ecosystem, name,
  version)` for stage 2b — but ONLY if that exact version has no content yet.
  Three constraints, all of which hold today and must not be relaxed:
  - **This stage stays OFFLINE.** `parse_dependencies` is sync and takes
    content. The fetch origin needs the registry, so the TASK resolves it;
    this stage carries it only when already cached. Network here would put
    I/O in front of the structure barrier and break a scan with no
    connectivity.
  - **DIRECT dependencies only** — the MANIFEST decides WHICH packages, and
    that is what keeps the transitive tree out. But the LOCKFILE decides
    WHICH VERSION: `clean_version` strips `^`, so a manifest-derived version
    is a range FLOOR indistinguishable from a pin (`^2.8.0` is stored as
    `2.8.0`). Look the direct deps up in the lockfile BY NAME; never
    enumerate it. No adapter reads one today — verified, zero references to
    `package-lock`/`Cargo.lock`/etc. in `crates/`.
  - **`DepVersion.source` is the manifest filename**, not the fetch origin.
    Do not reuse the name.

## 4. Failure modes

| input | this stage does |
|---|---|
| no `.gitmodules` | zero submodules. Normal, not an error. |
| `.gitmodules` malformed | parse what is valid, report the rest with the line. Do not abandon the whole file. |
| a declared submodule is NOT checked out | record the declaration, do not enqueue a `scan_repo` for a directory that is not there, and say which. A declared-but-absent submodule is a real and useful finding. |
| a manifest fails to parse | record the failure against the folder with the adapter name and the error. The repo's OTHER manifests still process — one bad `package.json` must not cost the repo its `Cargo.toml`. |
| a manifest names a workspace member that does not exist | record it; do not create a phantom folder row. |
| `git log` unavailable for subtree inference | zero subtree guesses. S3 already makes them optional. |
| an unreadable directory | record it and continue; the file set is incomplete and says so. |

**The manifest failure path is the one to get right.** Today's extraction runs
in a separate handler where a failure is invisible. Moved into the scan, a
swallowed manifest error silently costs a folder its commands, and nothing
counts it.

## 5. Verification

| test | mutation that must break it |
|---|---|
| `find_submodules` on a two-entry `.gitmodules` string returns both, with path and url | drop the second entry |
| a declared-but-absent submodule is reported and NOT enqueued | enqueue unconditionally |
| a submodule enqueues its own `scan_repo` | remove S2's enqueue |
| a file under a `.gitignore`d dir is absent from the file set | replace `build_walker` with a plain `WalkDir` |
| a nested `.gitignore` is honoured | set `parents(false)` on the walker |
| a `package.json` in `app/` produces commands on `app/`'s folder, not the repo root's | key the store on `repository_id` |
| six manifest-bearing folders in THIS repo produce six distinct command sets | key the store on `repository_id` — this is the measured case, 58 folders vs 36 repos |
| one malformed manifest does not cost the repo its other manifests | propagate the error instead of recording it |
| the manifest filename set is derived from the adapter registry | hardcode a filename list — the test greps for a literal `"package.json"` outside the adapters |

## 6. Watch out

**Everything about manifests already exists. Inventory before building.**
`ManifestAdapter` (`crates/senseid/src/adapters.rs`) is a deliberate sibling of
`LanguageAdapter`, created to replace ten hardcoded `if path == "package.json"`
sites. TEN ecosystems implement it (cargo, npm, pyproject, go, maven, gradle,
ruby, dotnet, composer, swiftpm). It provides `manifest_filenames`, `accepts`,
`ecosystem`, `parse_dependencies`, `detect_workspace_members`, `parse_commands`,
`is_workspace_root`, `stack_labels`, `infer_role`. It is CALLED today and
persists **572 commands across 58 folders and 4 ecosystems**.

v2 changes only WHERE IT RUNS — from a separate `libraries` task handler into
this stage, before the structure barrier — not what it extracts.

**Do NOT glob a second time for manifests.** This stage already walks every
file. Testing each against `accepts()` costs one string comparison; a separate
manifest glob would re-walk 48,665 files to find about 200, and would need its
own exclusion rules, which is a second place for them to drift.

**`DEFAULT_EXCLUDE_GLOBS` (`classifiers.rs:398`) and `.gitignore` now overlap.**
Several entries (`node_modules`, `dist`, `build`, `target`, `.next`,
`.svelte-kit`) are in most `.gitignore` files already. Do NOT bulk-delete them:
`require_git(false)` makes gitignore apply widely but a repo can still lack the
entry, and the `*.min.js` / `*.bundle.js` entries are NOT gitignore-covered —
they exist because a vendored Vite bundle under `artifacts/` contributed 540
call references to single-letter minified names, twice. Measure which entries
are now redundant using the two separate exclusion counters,
and remove only the ones proven so.

**A repo with no manifests is normal**, not a failure. Report zero and move on.

## 7. Definition of done

- Four pure functions unit-tested on literals, with no repo and no database.
- Submodules enqueue child `scan_repo`s; a declared-but-absent one is reported.
- Manifest facts land on the containing FOLDER; this repo produces six
  manifest-bearing folders and 572 commands, matching today's live count.
- The file set for this repo matches today's 48,665 within an explained delta.
- Manifest facts queryable: `folder_commands` shows six folders for this repo.
- No second walker and no hardcoded manifest filename list exist in the tree.
