# Stage 1 — scan root: which repositories exist

Whole-system spec: `docs/design/indexer-v2.md` §7g (R15), R14 (structure before
work). Depends on stage 0.

## 1. Purpose

Find the repository roots under a scanned directory and nothing else. This
stage knows which repos exist; it knows NOTHING about files. That division is
the whole point of the two-stage split — the moment scan root opens a repo's
contents, it has become scan repo and the exclusion rules have two homes.

Serves G2: "what is in this tree" is the first question a person asks of a
machine full of code, and it is answered before a single file is parsed.

## 2. Inputs and outputs

    find_git_roots(dir: &Path, exclusions: &RootExclusions) -> Vec<RepoRoot>   PURE-ish (fs read only)

    struct RepoRoot { abs_path: PathBuf, git_kind: GitKind }
    enum   GitKind  { Directory, File }   // File = a submodule checkout

| | |
|---|---|
| input | a directory to scan, plus root-level exclusions |
| output | repo roots, each with how its `.git` presents |
| IO | filesystem traversal only. **No database.** |

`save_repo` is a separate function and a separate requirement (S4) precisely so
that discovery can be tested with no database at all.

## 3. Requirements

- **S1** (R15). Glob for `.git` ENTRIES, matching both a directory and a FILE.
  A submodule checkout has a `.git` file pointing into the parent's
  `.git/modules/`, so globbing only for directories misses every submodule that
  is checked out.
- **S2** (R15). Apply root exclusions DURING traversal, not after. Descending
  into `node_modules` to discover you should not have is the cost this stage
  exists to avoid.
- **S3.** Do not descend into a repo root once found. Whatever is inside it is
  stage 2's, including any nested repo — which stage 2 discovers and enqueues
  itself (see 02-scan-repo S2).
- **S4** (R14). `save_repo(root) -> RepositoryId` upserts `sensei.repositories`
  keyed on `repo_key` (the NORMALIZED remote), and the repo-root FOLDER row
  carrying `repository_id`. These are two tables and both are required:
  `repositories` is the identity that survives a re-clone or a move,
  `folders.abs_path` is where it sits today. Two clones of one remote are TWO
  folders and ONE repository.
- **S5.** A repo with no remote gets `repo_key = NULL`. The column is UNIQUE
  with nulls distinct, so many local-only repos coexist. Do NOT mint a
  synthetic key from the path — that is a fabricated identity, and it would
  make a repo's identity change when it moves.

### What this stage explicitly does NOT do

Reading `.gitmodules` (that is stage 2 — the file is INSIDE a repo), any file
discovery, any manifest reading, any parsing.

## 4. Failure modes

| input | this stage does |
|---|---|
| the scanned directory does not exist | `Err`, named. Not an empty root list — an empty list says "no repos here", which is a different fact. |
| a `.git` entry is unreadable (permissions) | record the root with a reason and continue. It exists; we could not read it. |
| a `.git` FILE whose gitdir pointer is dangling | still a root. A broken submodule checkout is a real state, and stage 2 will report what it finds. |
| the remote cannot be read | `repo_key = NULL` (S5). Never a synthetic key. |
| two roots normalise to one `repo_key` | expected — two clones. Two folder rows, one repository row. Not an error. |
| a symlink loop | bounded traversal; report the loop rather than hanging. |

## 5. Checkpoint output

    {"stage":"01-scan-root","at":"<iso8601>","scanned_dir":"…",
     "roots_found":68,"git_dir":66,"git_file":2,
     "repos_upserted":68,"repos_new":0,"repos_with_remote":66,"repos_local_only":2,
     "excluded_dirs_skipped":<n>,
     "sample":[{"abs_path":"…/sensei","git_kind":"Directory",
                "repo_key":"github.com/sensei-hq/sensei","folder_id":"…"}]}

`git_file` counts submodule checkouts found at root level; if it is 0 on a tree
you know has submodules, S1 regressed to directories-only.

## 6. Verification

| test | mutation that must break it |
|---|---|
| a fixture tree with a `.git` DIR and a `.git` FILE returns both | change the glob to directories only |
| a repo inside an excluded dir is NOT returned | move the exclusion check to after traversal |
| traversal does not descend below a found root | remove S3's prune |
| two fixture repos with the same remote produce ONE `repositories` row and TWO folder rows | key `repositories` on path instead of `repo_key` |
| a remote-less repo gets `repo_key IS NULL`, and two of them coexist | mint a key from the path |
| `find_git_roots` runs with no database connection | make it take a `&PgPool` |

That last one is a design test, not a nicety: if discovery cannot run without a
database, stages 1–3 stop being cheap to test, which is the property the
sequence was ordered around.

## 7. Watch out

**Globbing for `.git` finds far fewer roots than today's scan tracks folders.**
That is intended and was agreed: a large number of directories currently stored
as "non-git repos" will stop being roots. They are not lost — they are folders
inside some root, or they are outside the scan. Count the difference and put it
in the checkpoint; do not let it be discovered later as an apparent regression.

**`repo_key` normalisation already exists** and is documented on the
`repositories` table: `git@host:Org/Repo.git` and `https://host/Org/Repo` both
become `host/org/repo`, scheme/creds/port/`.git` stripped, host lowercased.
Use it. Do not write a second normaliser.

## 8. Definition of done

- `find_git_roots` is pure of the database and unit-tested against a fixture
  tree containing a `.git` dir, a `.git` file, an excluded dir, and a nested
  repo.
- `save_repo` upserts both tables idempotently — running it twice changes no row.
- Checkpoint line written with a rendered sample.
- The root-count delta against today's folder set is measured and explained.
