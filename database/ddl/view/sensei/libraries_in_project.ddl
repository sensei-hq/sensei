set search_path to sensei, extensions;

create or replace view libraries_in_project as
select distinct
       fp.project_id
     , rl.library_id
     , l.name
     , l.ecosystem
     , l.kind          as library_kind
     , l.description
       -- page_count is per VERSION now (S7b), so it is DERIVED here rather
       -- than stored on `libraries`. The latest version is what a project
       -- without a matching pin would be served, so that is the count shown.
     , coalesce((select v.page_count from library_versions v
                  where v.library_id = l.id and v.is_latest), 0) as page_count
     , l.icons
     , l.tags
  from referenced_libraries rl
  join folder_projects      fp on fp.folder_id = rl.folder_id
  join libraries            l  on l.id = rl.library_id;

comment on view libraries_in_project is
'Unique set of libraries used across all folders in each project.
Joins referenced_libraries through folder_projects to projects, deduplicates by (project_id, library_id).

Membership comes from `folder_projects`, never from `folders.project_id`. The
old column was settable per folder, so the folders of one repository could name
different projects; `project_repositories` now holds membership and
`folder_projects` resolves it through `folders.repository_id`.

`folder_projects` IS MULTI-VALUED — a repository is keyed on its remote, so one
repository can serve more than one project (measured 2026-10-02: 8 of them do,
two projects each). THE `SELECT DISTINCT` ALREADY OWNS THAT, and it is why no
pre-collapse sub-select was added: the distinct key is
(project_id, library_id) — every other output column is functionally determined
by library_id — so a folder in two projects yields one row per project and never
a second row within a project. There is no aggregate here to double.

The output therefore GREW, and the growth is the point. Measured 2026-10-02 on
the live DB: 4,991 rows under the `folders.project_id` form, 5,410 under this
one — a strict superset, 419 added and none dropped (symmetric EXCEPT ALL:
old_not_new 0, new_not_old 419). The grain held: 5,410 rows, 5,410 distinct
(project_id, library_id). Every one of the 419 was classified, 0 left over —
each reaches its project only through a folder whose repository serves more than
one project, and lands in one of the 14 projects that share a repository. Each
is a library genuinely referenced by a folder of a repository that project owns;
the old column showed it to only one of the two owners.

The old `where f.project_id is not null` guard is gone because the inner join to
`folder_projects` subsumes it — a folder under no tracked repository contributes
no row rather than an invented project. Measured 2026-10-02: 0 of 13,724 folders
carry a null `repository_id` and 0 of 7,025 referenced_libraries rows fall out
that way, so nothing was dropped. The column stays NULLABLE, so that zero is the
current state, not a structural guarantee.

Consequence for fixtures: a folder inserted with a NULL `repository_id` and then
wired to a project by `UPDATE folders SET project_id` produces NO row here. A
fixture must create a `repositories` row, a `project_repositories` row, and set
`folders.repository_id`.

`folder_projects` is referenced directly rather than restated, and NOT wrapped in
a CTE — in `graph_nodes` the identical SQL spelled as a CTE stopped the planner
parameterising the view by the consumer''s key. Plan shape for
`WHERE project_id = $1`, EXPLAIN at `max_parallel_workers_per_gather = 0` on
2026-10-02: Seq Scan on the 261-row `project_repositories` -> Bitmap Index Scan
on `folders_repository_id_idx` -> Index Only Scan on `referenced_libraries_pkey`
-> Index Scan on `libraries_pkey`. No sequential scan of `folders` or
`referenced_libraries`. (No claim is made here about what the
`folders.project_id` form planned as: swept across all 250 projects it has no
single shape — a Nested Loop with Memoize in the large majority, a Hash Join in
a handful — so any one description of it would be a minority case stated as the
rule.) The `left join projects` inside
`folder_projects` is eliminated outright, because no column of it is selected
here. No wall-clock is quoted: `folders` is large enough relative to
shared_buffers that repeated identical runs are not comparable.';
