set search_path to sensei, extensions;

create table if not exists file_complexity (
  repository_id  uuid         not null references sensei.repositories(id) on delete cascade
, path           text         not null
, functions      integer      not null
, complex        integer      not null
, max_cyclo      integer      not null
, measured_sha   text         not null
, measured_at    timestamptz  not null default now()
, primary key (repository_id, path)
);

comment on table file_complexity is
'Per-file function complexity as local `qlty metrics --functions` measures it (user,
2026-10-27: rely on local qlty rather than re-deriving what it measures), for the Code
map''s Complexity lens (M17). Measured at the repository''s HEAD in a temporary worktree
against the pinned qlty ruler — never in the working copy. Replaced whole per
repository at each measurement; one HEAD per repository, so every row of a repository
carries the same `measured_sha`.';
comment on column file_complexity.repository_id is 'The repository measured.';
comment on column file_complexity.path          is 'Repository-relative path, as git and qlty name it.';
comment on column file_complexity.functions     is 'Functions and methods qlty counted in the file.';
comment on column file_complexity.complex       is 'Of those, how many exceed cyclomatic complexity 10.';
comment on column file_complexity.max_cyclo     is 'The largest cyclomatic complexity of any function in the file.';
comment on column file_complexity.measured_sha  is 'The HEAD commit measured; a scan re-measures only when HEAD moved.';
comment on column file_complexity.measured_at   is 'When it was measured.';
