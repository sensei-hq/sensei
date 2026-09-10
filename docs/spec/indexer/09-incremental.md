# Stage 9 — incremental: changed files back to repos

Whole-system spec: `docs/design/indexer-v2.md` R14 (incremental mode), R15
(§7g, the root set), R6, R10.5. Depends on stage 7.

## 1. Purpose

A change arrives as a set of paths. Turn it into the smallest correct amount of
work: match each path to its repo, update the file and folder rows, and
re-parse only what changed.

This is the INVERSE of stages 1–3. Those walk down from a root to files; this
walks up from a file to its root.

## 2. Inputs and outputs

    match_repos(paths: &[PathBuf], known_roots: &[RepoRoot]) -> MatchResult   PURE
    classify(old: &FileRow, new: &FileFsFacts)               -> ChangeKind    PURE
    apply_incremental(MatchResult)                           -> ()            IO

Both decision functions are PURE and take the root set as data, which is what
makes the longest-prefix rule testable without a filesystem.

## 3. Requirements

- **S1** (R15). **LONGEST PREFIX WINS.** A file under `repo/submodule/x.rs`
  belongs to the SUBMODULE, not the parent. This requires submodule roots to
  be in `known_roots`.
- **S2** (R15). `known_roots` is read from the DATABASE, never from a scan
  root run. Submodules are discovered by `scan_repo` (stage 2 S2), so the
  complete root set exists only after the scan-repo wave drains — and it is
  persisted, which is what makes S1 correct for submodules.
- **S3** (R15). A path matching NO known root ESCALATES to `scan_root`. A new
  repo appeared; do not guess a parent for it.
- **S4** (R14). Classify from OLD plus NEW — the pairing is what makes a change
  classifiable rather than guessable:

  | detected | file/folder rows | reparse? |
  |---|---|---|
  | content changed, path same | touch `mtime` + `content_hash` | YES |
  | path changed, content same | UPDATE the path, `id` unchanged | see S5 |
  | path + content changed | update both | YES |
  | added | create the row | YES |
  | removed | reconcile (R10.8), never a prefix DELETE | reconcile |
  | touched, hash identical | refresh `mtime` only | NO |

- **S5** (R14). A rename is NOT always free of re-parsing. In Rust and TS/JS
  the MODULE PATH is derived from the file path and is a segment of every fqn
  the file declares. A rename that CHANGES the module path re-mints every
  declaration even though the bytes are identical; one that does not (a
  case-only change, a move preserving the module) updates the file row and
  stops. That is a property of the fqn grammar, not of the storage choice.
- **S6** (R14). `content_hash` decides re-parse and already exists on the file
  row, already gating this today in `plan_reindex`. Reuse it.
- **S7** (R10.5). Deletion goes through reconcile, one file at a time. A folder
  deletion is N file reconciles. Never a path-prefix `DELETE` — the cascade
  reason in R10.2 applies unchanged.
- **S8.** Ignore rules apply here exactly as in stage 2, via the SAME
  `build_walker`. The fs-watcher receives raw FSEvents that know nothing about
  `.gitignore`; a documented add/prune churn loop was caused by the watcher and
  the scan disagreeing about what belongs in the index.
- **S9.** **A changed MANIFEST or LOCKFILE retriggers the manifest pass** (02
  S5–S6d). A dependency change is invisible to the file-parse path: no `.rs`
  or `.ts` file changed, so nothing else notices, and the graph keeps
  yesterday's dependency set until a full re-scan happens to run.
- **S10.** **THE FAN-OUT IS ASYMMETRIC, and this is the part to get right.**

  | changed | retrigger |
  |---|---|
  | a manifest (`Cargo.toml`, `package.json`) | THAT folder only |
  | a lockfile | **EVERY folder whose manifest resolves to it** (02 S6c) |

  A lockfile serves a subtree, so a change to it changes the pins of every
  manifest below that has no nearer lock. Measured here: a change to
  `/Cargo.lock` retriggers **9 manifests** — the root plus 8 workspace
  members under `crates/`. A change to `app/src-tauri/Cargo.lock` retriggers
  exactly 1.

  Getting this wrong is silent in the direction that matters: retrigger only
  the lockfile's own folder and eight crates keep stale pins, with nothing
  counting them. Compute the served set with the SAME resolution function
  stage 2 uses (S6c) — inverted, not reimplemented. Two implementations of
  "which manifests does this lockfile serve" will disagree.

- **S11.** A manifest or lockfile change does **NOT** re-parse source files.
  The dependency set changed; the code did not. Re-parsing 48,646 files
  because a version bumped is the waste R14's structure/work split exists to
  avoid. The exception is a PACKAGE RENAME, which changes every fqn in the
  package and is already a full re-index by R10.5 — detected by the manifest
  reader comparing `name` against what the graph was built with, not by this
  stage.

## 4. Failure modes

| input | this stage does |
|---|---|
| a path under no known root | escalate to `scan_root` (S3), do not attach it to the nearest ancestor. |
| a path under two roots | longest prefix wins (S1). Deterministic, not first-match. |
| a root that no longer exists on disk | the repo was removed; reconcile its files. Do not prune the `repositories` row — the identity survives a local checkout going away. |
| a file whose old row is missing | treat as ADDED. Do not fail; a missed create is recoverable, a wrong delete is not. |
| a rename detected as delete+add by the watcher | correct but wasteful. Accept it — R10.5 says a rename IS a deletion plus an addition, so the result is right either way. |
| the hash is unchanged but `mtime` moved | refresh `mtime`, no re-parse (S4 last row). |
| a burst of thousands of events | batch by repo before applying. Do not enqueue one task per event. |

## 5. Verification

| test | mutation that must break it |
|---|---|
| a path under a submodule matches the SUBMODULE, not the parent | change longest-prefix to first-match |
| `match_repos` runs on literal path lists with no filesystem and no database | give it a `&PgPool` |
| a path under no root escalates | attach it to the nearest ancestor |
| each of the six change kinds in S4 is classified correctly from an old/new pair | collapse any two rows |
| a rename that CHANGES the module path re-parses; one that does not, does not | make all renames skip re-parse |
| a removed file goes to reconcile, not `DELETE` | replace the reconcile call |
| a folder deletion becomes N file reconciles | issue a prefix delete |
| a `.gitignore`d path that reaches the watcher is skipped | bypass `build_walker` |
| a changed `Cargo.toml` retriggers the manifest pass for ITS folder only | retrigger the whole repo |
| a changed `/Cargo.lock` retriggers all 9 manifests it serves, not just the root | retrigger only the lockfile's own folder — 8 crates keep stale pins and nothing counts them |
| a changed `app/src-tauri/Cargo.lock` retriggers 1, and the root's 9 are untouched | resolve to the repo-root lockfile instead of the nearest |
| a manifest/lockfile change re-parses ZERO source files | trigger a re-index on a version bump |
| the served-set computation calls stage 2's resolver, inverted — not a second copy | write a second "which manifests does this lock serve" |
| an unchanged hash with a moved `mtime` does NOT re-parse | re-parse on `mtime` |

Uses the mutation fixture from stage 7 — a temp git repo the test builds and
mutates. It is specified there and referenced here, not duplicated.

## 6. Watch out

**The root set is not complete until the scan-repo wave drains** (S2). Reading
it from scan root's in-memory output would miss every submodule and quietly
attach submodule files to their parent repo. Read the database.

**The watcher and the scan must never disagree about what belongs in the
index.** They did, and it cost a permanent add/prune churn loop: the watcher
enqueued 131 generated i18n message files, the scan correctly did not see them,
so they landed in `plan.removed`, had their nodes deleted, and the next build
re-added them. `build_walker` at `max_depth(1)` is the existing shared answer.

**A burst of events is normal** — a branch switch touches thousands of files.
Batch by repo; a per-event task storm is how the queue got 350-file bursts
every five minutes.

## 7. Definition of done

- `match_repos` and `classify` are pure, tested on literals.
- Longest prefix proven against a submodule fixture, with rejected roots in the
  sample.
- All six change kinds classified and tested.
- Rename splits correctly on whether the module path changed.
- Deletion goes through reconcile everywhere; no prefix DELETE exists.
- Watcher and scan share one ignore path.
