set search_path to sensei, extensions;

create table if not exists code_duplicates (
  repository_id  uuid         not null references sensei.repositories(id) on delete cascade
, hash           text         not null
, identical      boolean      not null
, mass           real         not null
, lines          integer      not null
, copies         jsonb        not null
, measured_sha   text         not null
, measured_at    timestamptz  not null default now()
, primary key (repository_id, hash)
);

comment on table code_duplicates is
'Code that local `qlty smells` finds more than once in a repository (user, 2026-10-27:
duplicates come from the scan that already finds them, not a similarity search run per
question). One row per group of copies. Measured at the repository''s HEAD in a temporary
worktree against the pinned qlty ruler, with the Complexity lens'' measurement; replaced
whole per repository, so every row of a repository carries the same `measured_sha`.';
comment on column code_duplicates.repository_id is 'The repository measured.';
comment on column code_duplicates.hash          is 'qlty''s structural hash: the copies of one piece of code share it.';
comment on column code_duplicates.identical     is 'True when the copies are identical; false when qlty found them similar.';
comment on column code_duplicates.mass          is 'qlty''s mass — the size of the duplicated syntax; groups are ranked by it.';
comment on column code_duplicates.lines         is 'Lines in the longest copy.';
comment on column code_duplicates.copies        is 'Every copy: [{path, start, end}], repository-relative paths, inclusive lines.';
comment on column code_duplicates.measured_sha  is 'The HEAD commit measured; a scan re-measures only when HEAD moved.';
comment on column code_duplicates.measured_at   is 'When it was measured.';
