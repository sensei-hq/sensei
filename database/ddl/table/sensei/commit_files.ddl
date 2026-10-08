set search_path to sensei, extensions;

create table if not exists commit_files (
  repository_id            uuid        not null
, sha                      text        not null
, path                     text        not null
, lines_changed            integer
, primary key (repository_id, sha, path)
, constraint commit_files_commit_fk foreign key (repository_id, sha)
      references sensei.commits(repository_id, sha) on delete cascade
);

-- The per-path read: Ownership ("who touches this file, and when did they
-- stop") and churn ("how far has this file moved"). Both filter
-- `(repository_id, path)` and then join `sensei.commits` on sha for the
-- `authored_at` window, so sha trails the two filter columns and the join keys
-- come out of the index with no heap visit. `lines_changed` is then the only
-- column left and it is 4 bytes, so INCLUDEing it makes the churn sum
-- index-only too, for at most 6 MB across the projected ~738,000 machine-wide
-- rows once tuple alignment is counted. The table is insert-only, so autovacuum
-- keeps the visibility map set and index-only stays true instead of decaying.
-- Measured on PG 17 over 40,000 synthetic rows: both the raw per-path read and
-- the Ownership-shaped join plan as `Index Only Scan using
-- commit_files_path_idx`, with the join reaching `commits` by its primary key.
--
-- This index is close to a second copy of the table, and that is the price of
-- serving a per-path read at all rather than carelessness: the table is almost
-- entirely key, so the three key columns are most of the row already and the
-- INCLUDE is the only part of the size this choice actually added.
--
-- There is no second index. The PK already begins `(repository_id, sha)`, which
-- is the per-commit direction, and the retention cascade is exactly that
-- lookup: pruning a parent issues `delete from commit_files where
-- repository_id = ? and sha = ?`, measured taking `commit_files_pkey`. Hidden
-- coupling wants no index at all — it reads a whole window and groups by
-- commit, and at 400,000 rows with a window selecting 0.36% of commits the
-- planner still chose a parallel seq scan and a hash join. A bulk read is the
-- correct plan for it, so an index added on its behalf is one nothing uses.
create index if not exists commit_files_path_idx
    on commit_files (repository_id, path, sha) include (lines_changed);

comment on table commit_files is
'Which PATHS a commit touched, and how far each one moved — the per-touch git
fact behind Hidden coupling (co-change), Ownership and churn.

- **What it is.** One row per (repository, commit, path). Insert-only and
  immutable: a commit that has been walked never changes, so a re-walk is
  `on conflict do nothing`, nothing here is ever updated, and nothing is ever
  pruned by reachability. Written per CHECKOUT — the cursor is
  `commit_scans.folder_id` — but keyed per REPOSITORY, so a second checkout of
  the same repo re-inserts the same rows as a pure no-op instead of duplicating
  them or racing the first checkout to delete them.
- **How to use it.** Co-change: group by sha, take the paths pairwise, and drop
  any commit touching more than 50 files (a sweep, not coupling — 81.5% of the
  588,865 naive pairs in `sensei` come from the 53 commits above that line).
  Ownership and churn: filter `(repository_id, path)`, join `sensei.commits` for
  `authored_at` and `author_email`, window on `authored_at` (default 90 days,
  configurable). Any read that is about files on disk joins `sensei.files` by
  path, and THAT join is the filter — see below.
- **What it is NOT.** Not a graph edge and not a file reference. There is no
  `file_id` and no `node_id`; a path here is a git string that is valid only for
  the commit it sits on. Not a time series either — it carries no timestamp of
  its own, because `commits.authored_at` already owns that fact and a copy here
  would be a second writer for it.
- **Who writes it.** The git-history scan, from
  `git log --no-merges --numstat -z` over the TIP SET
  (`--branches --remotes --tags`; never HEAD alone, never `--all`). Measured in
  `sensei`: 4,060 non-merge commits, 24,588 touches, median 2 files per commit,
  p95 20, max 326. Retention deletes from `sensei.commits` at 400 days and these
  rows follow through the cascade; the floor `retention >= max(window)` is
  enforced so a window can never outrun the data behind it.

**The natural key is load-bearing.** `(repository_id, sha, path)` is the primary
key and there is deliberately no surrogate `id`. Nothing references a row here,
so a surrogate would exist only to be returned — and it could not be. The
writer''s idempotent insert is `on conflict do nothing`, and
`on conflict do nothing returning id` returns NOTHING on conflict, so a caller
that needed the id would have to read every already-known row back, or escalate
to `on conflict do update` and write a fresh row version for all ~738,000 of
them on every sweep. That is a tableful of dead tuples burned per re-walk to
maintain a column with no referent. With the natural key a re-walk over
already-recorded commits writes nothing at all.

`repository_id` is not separately a foreign key to `sensei.repositories`. The
composite FK to `sensei.commits` carries it, `commits` holds that reference, and
a repository delete reaches these rows down the chain. A second constraint would
be a second index obligation for a guarantee already made.

**No `file_id`, and that is not an omission.** 4,298 of the 6,654 paths ever
touched in `sensei` (65%) no longer exist on disk, so they have no
`sensei.files` row and the column would be NULL for most of the table. Worse, it
would make a git fact depend on whether the indexer has run: these rows are
insert-only, so a path written before its file was indexed would hold NULL
forever, and the column would be recording scan ORDER rather than truth. Resolve
by path at read time instead. EXISTENCE on disk is the filter — a path that is
gone fails the join to `sensei.files` and so cannot reach a diagram — and a
filter belongs in the query, never frozen into the row.';

comment on column commit_files.repository_id
     is 'The repository the commit belongs to — the fact grain, and half of the composite FK to `sensei.commits`. Keyed on the repository rather than the checkout so two clones converge on one set of rows: 10 repositories on this machine have more than one anchor folder, and a per-checkout key would give each of them a private copy of the same history.';
comment on column commit_files.sha
     is 'The commit, as the full 40-character git object name. `text`, matching `repository_metrics.commit_sha`; `char(40)` would buy nothing in Postgres and would blank-pad anything shorter. Purely the join key to `sensei.commits` — `authored_at` and `author_email` are stored only there, so this table cannot disagree with them.';
comment on column commit_files.path
     is 'Repository-relative path with POSIX separators, exactly as git reported it FOR THIS COMMIT — the path the file had at that commit, which is what a later join against `sensei.files` needs. Read from `--numstat -z`, where paths arrive as NUL-terminated raw bytes: no C-quoting, and no `a/{x => y}/b` rename pseudo-path. Both of those are live defects in the churn path this table replaces — 2,087 of 24,496 records (8.5%) stored a rename pseudo-path that never existed on disk, and 30 more stored octal escapes. A rename contributes ONE row, under the new path. This is not an identity across renames; tracking that is the graph''s job, not git''s.';
comment on column commit_files.lines_changed
     is 'Lines added plus lines deleted for this path in this commit. NULLABLE, and null means UNKNOWN — never zero. `git --numstat` prints `-` for both counts on a binary file, and `churn.rs:234` currently reads that as `add.parse().unwrap_or(0) + del.parse().unwrap_or(0)`, which reports replacing a 4 MB asset as a no-op edit and quietly drags every mean down: that is the bug this column exists not to repeat. The TOUCH is still a fact — the row is here, so co-change and Ownership counts are unaffected — and only its magnitude is unknown. Readers let the null propagate (`sum` and `avg` skip it) and count `lines_changed is null` when they need to report how much is unmeasured. Never `coalesce(lines_changed, 0)`. `integer`, not `bigint`: no single file-touch approaches 2.1 billion lines, and `sum(integer)` already returns `bigint`, so roll-ups cannot overflow.';
