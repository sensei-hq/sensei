set search_path to sensei, extensions;

create table if not exists commits (
  repository_id            uuid        not null references sensei.repositories(id) on delete cascade
, sha                      text        not null
, authored_at              timestamptz not null
, author_email             text
, primary key (repository_id, sha)
);

-- The window read, which is the only read this table has: "the commits of ONE
-- repository inside the last N days". Leading on repository_id and ranging on
-- authored_at makes that a single range scan. The primary key indexes
-- (repository_id, sha), so without this one the window is a filter over the
-- repository's entire history — 4,060 rows for sensei, ~121,780 machine-wide.
-- The retention prune is written per-repository so it reads this index too,
-- rather than earning a second index on authored_at alone.
create index if not exists commits_repository_id_authored_at_idx
    on commits(repository_id, authored_at desc);

comment on table commits is
'Every commit of a REPOSITORY — the commit-grain facts the co-change, ownership
and bus-factor derivations read.

- **What it is.** One row per non-merge commit on the TIP SET: `git log
  --branches --remotes --tags`, never `HEAD` alone and never `--all`. HEAD is
  one branch, not the repository; `--all` sweeps `refs/stash` and
  `refs/original`, local debris that two clones will never agree on. On the tip
  set they do agree — of the 4 co-keyed checkout pairs measured on this machine,
  3 produce byte-identical commit sets. Merges are excluded: a merge''s numstat
  is a diff against one arbitrary parent, so its touched files record parent
  order rather than a change anyone made. Measured on sensei: 4,060 commits,
  24,588 file touches, median 2 files per commit, p95 20, max 326. Machine-wide
  ~121,780 commits, ~738,000 `commit_files`, ~185 MB.
- **How to use it.** Join `sensei.commit_files` on `(repository_id, sha)` and
  range on `authored_at` for the window — default 90 days, configurable.
  Co-change pairs the files of one commit; ownership groups the same window by
  `author_email` (configurable commits / lines / recency, default commits).
  Drop commits over the bulk cap of 50 files BEFORE pairing: 81.5% of sensei''s
  588,865 naive co-change pairs come from the 53 commits that touch more than
  50 files, so the cap is what makes the pair count usable rather than a tuning
  knob.
- **What it is NOT.** Not a mirror of the git object graph — a row outlives the
  commit''s reachability, see below. Not scan state: the cursor is
  `sensei.commit_scans`, keyed on `folder_id`, because a scan runs per CHECKOUT
  while these facts are per REPOSITORY. Not the churn metric''s source — churn
  walks git live on committer-days and persists nothing.
- **Who writes it.** The `ScanGitHistory` task, insert-only
  (`on conflict do nothing`), running once per checkout but converging on one
  row set per repository. Nothing else writes here and no row is ever updated.

INSERT-ONLY, AND NOTHING IS PRUNED BY REACHABILITY. v1 deleted rows missing from
`rev-list HEAD`, and that was unsound: the facts are repository-grain while scans
run per checkout, and 10 repositories on this machine have more than one anchor
folder — each scan would prune the other''s commits and re-walk them, forever.
The prune was also redundant. 4,298 of the 6,654 paths ever touched in sensei
(65%) no longer exist, so they have no `sensei.files` row and cannot reach a
diagram. EXISTENCE on disk is the filter, not reachability. The one deletion is
the age-based retention prune — default 400 days, with an ENFORCED floor of
`retention >= max(window)`, so a prune can never eat the window a derivation is
about to read.

AUTHORED_AT, NOT THE COMMITTER DATE. The author date survives a rebase and a
cherry-pick, so one logical change buckets into the same window through two
checkouts; the committer date is rewritten by both, and the two checkouts would
disagree about when the same change happened. This DIVERGES from `churn.rs`,
which buckets on the committer date (`%cd`). Stated here rather than hidden: the
same commit can land on different days in the two systems, and that is the cost
of making a repository-grain fact stable across checkouts.';

comment on column commits.repository_id
     is 'FK to sensei.repositories — the grain, and the reason two checkouts of one remote converge on a single row set instead of each keeping its own. ON DELETE CASCADE: these facts are about the repository and none of them outlive it.';
comment on column commits.sha
     is 'The full commit object id as git reports it (`%H`), verbatim and never truncated. No length constraint — 40 hex characters under sha-1, 64 under sha-256. Unique only WITHIN a repository, hence the composite primary key: a fork, a mirror or a subtree copy legitimately carries the same sha in a different repository.';
comment on column commits.authored_at
     is 'The AUTHOR date (`%aI`), not the committer date — it survives a rebase and a cherry-pick, so one logical change buckets into the same window through two checkouts. See the table comment for the deliberate divergence from churn.rs. NOT NULL: a commit object always carries an author date, so a value that will not parse is a scan error, never a substituted now().';
comment on column commits.author_email
     is 'The author email as git reports it (`%ae`) — raw, with no mailmap, no lowercasing and no alias folding. Coalescing two addresses into one person is an ownership-READ concern; the stored row stays what git said. NULL when the commit records no author email at all (`<>`): that is UNKNOWN, and it must not become an empty-string pseudo-author that the ownership group-by would count as a contributor. Same rule as a binary file''s `commit_files.lines_changed`.';
