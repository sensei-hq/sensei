set search_path to sensei, extensions;

create table if not exists commit_scans (
  folder_id                uuid        primary key references sensei.folders(id) on delete cascade
, repository_id            uuid        not null references sensei.repositories(id) on delete cascade
, tips                     text[]      not null default '{}'
, scanned_at               timestamptz not null default now()
);

-- Covering index for the repository_id FK: a repository delete cascades here, and
-- "which checkouts have walked this repository" is the one query that does not
-- already have the primary key in hand.
create index if not exists commit_scans_repository_id_idx
    on commit_scans(repository_id);

comment on table commit_scans is
'How far the git-history walk has got, per CHECKOUT — the cursor for the facts in
`sensei.commits` and `sensei.commit_files`.

- **What it is.** One row per repo-anchor folder that has been walked at least
  once. `tips` holds the tip shas that walk covered, so the next walk asks git
  only for what is new: `git rev-list <current tips> --not <stored tips>`.
- **How to use it.** Resolve the current tip set with `--branches --remotes
  --tags`. Never HEAD alone — it sees one branch of a checkout that has many.
  Never `--all` — it sweeps `refs/stash` and `refs/original`, which are not
  history anyone authored. Walk the difference, insert the facts, then write the
  new tip set back here.
- **What it is NOT.** It is not the facts, and it governs nothing about their
  lifetime. They are keyed on `repository_id`, insert-only, and never pruned by
  reachability. Losing this row — or the checkout it points at — loses the
  position, not the history: the next walk simply re-walks, and every row it
  re-inserts is absorbed by `ON CONFLICT DO NOTHING`.
- **Who writes it.** The git-history scan handler, in the same transaction that
  commits the walk''s rows and ONLY when that walk succeeded. A failed walk
  leaves the previous tips standing, so the next run retries the same range
  instead of stepping over it.

## Why the cursor is per CHECKOUT when the facts are per REPOSITORY

A scan runs in a working directory; a commit belongs to a repository. Two
checkouts of one repository — `develop` in one folder, `main` in another — are
two scans at two positions, and 10 repositories on this machine have more than
one anchor folder. Keying this table on the repository would hand those pairs
ONE cursor to fight over: each walk would advance it past refs the other cannot
see, and each would then be told it had already covered them.

The predecessor shipped a worse form of exactly that. v1 pruned commits by
`rev-list HEAD` reachability — repository-grain rows deleted on checkout-grain
evidence — so two anchors of one repository would prune and re-walk each other
forever. Splitting the grain settles it at the key: a per-folder cursor can be
wrong about its own folder and nothing else, while the facts it feeds stay
shared. The prune it replaced was never needed either; EXISTENCE on disk is the
filter, not reachability. Of 6,654 paths ever touched in this repository, 4,298
(65%) no longer exist, so they have no `sensei.files` row and cannot reach a
diagram however long their commits are kept.

## When a tip disappears

A force-push, a deleted branch or a rewritten history leaves a stored sha that
is no longer an object. Nothing here is repaired and nothing anywhere is
deleted. Filter the stored tips to the shas git can still resolve before passing
them as exclusions — `rev-list --not <missing sha>` aborts the whole command —
and the walk re-covers that range on the shas that remain. Re-walking costs
time; it cannot produce a wrong row, because every row it writes is keyed on the
sha that produced it.';

comment on column commit_scans.folder_id
     is 'The CHECKOUT this cursor belongs to — the repo-anchor folder the walk ran git in (FK sensei.folders, ON DELETE CASCADE). PRIMARY KEY rather than repository_id because two checkouts of one repository must each keep their own position; see the table comment. The cascade drops the position only: the facts are keyed on the repository and outlive any checkout being pruned.';
comment on column commit_scans.repository_id
     is 'The repository the walk wrote facts for (FK sensei.repositories, ON DELETE CASCADE). NOT NULL — a commit is repository-grain, so a folder whose repository the scanner has not resolved yet (`folders.repository_id` is nullable) is not scannable and gets no row here at all, never a row carrying an invented repository.';
comment on column commit_scans.tips
     is 'The tip shas the last successful walk covered, from `git rev-list --branches --remotes --tags`. Handed to the next walk as `--not <tips>` so it reads only new commits.

Stored SORTED, so an unchanged tip set compares equal and the walk can stop before spawning git. A sha that no longer resolves must be dropped from the exclusion list rather than passed through — see the table comment.

`{}` is a real state: the walk ran and the repository had no branch, tag or remote. No row at all means never walked.';
comment on column commit_scans.scanned_at
     is 'When the last successful walk finished. Advanced together with `tips`, in one write — so the pair can never claim coverage the facts do not have.';
