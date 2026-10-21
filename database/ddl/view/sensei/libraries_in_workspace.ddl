set search_path to sensei, extensions;

create or replace view libraries_in_workspace as
select distinct
       fp.workspace_id
     , rl.library_id
     , l.name
     , l.ecosystem
     , l.kind          as library_kind
     , l.description
       -- page_count is per VERSION now (S7b), so it is DERIVED here rather
       -- than stored on `libraries`. The latest version is what a workspace
       -- without a matching pin would be served, so that is the count shown.
     , coalesce((select v.page_count from library_versions v
                  where v.library_id = l.id and v.is_latest), 0) as page_count
     , l.icons
     , l.tags
  from referenced_libraries rl
  join folder_workspaces    fp on fp.folder_id = rl.folder_id
  join libraries            l  on l.id = rl.library_id;

comment on view libraries_in_workspace is
'Unique set of libraries used across all folders in each workspace.
Joins referenced_libraries through folder_workspaces to workspaces, deduplicates by (workspace_id, library_id).

Membership comes from `folder_workspaces`, never from `folders.workspace_id`. The
old column was settable per folder, so the folders of one repository could name
different workspaces; `repositories_in_workspaces` now holds membership and
`folder_workspaces` resolves it through `folders.repository_id`.

`folder_workspaces` IS MULTI-VALUED — a repository is keyed on its remote, so one
repository can serve more than one workspace (measured 2026-10-02: 8 of them do,
two workspaces each). THE `SELECT DISTINCT` ALREADY OWNS THAT, and it is why no
pre-collapse sub-select was added: the distinct key is
(workspace_id, library_id) — every other output column is functionally determined
by library_id — so a folder in two workspaces yields one row per workspace and never
a second row within a workspace. There is no aggregate here to double.

The output therefore GREW, and the growth is the point. Measured 2026-10-02 on
the live DB: 4,991 rows under the `folders.workspace_id` form, 5,410 under this
one — a strict superset, 419 added and none dropped (symmetric EXCEPT ALL:
old_not_new 0, new_not_old 419). The grain held: 5,410 rows, 5,410 distinct
(workspace_id, library_id). Every one of the 419 was classified, 0 left over —
each reaches its workspace only through a folder whose repository serves more than
one workspace, and lands in one of the 14 workspaces that share a repository. Each
is a library genuinely referenced by a folder of a repository that workspace owns;
the old column showed it to only one of the two owners.

The old `where f.workspace_id is not null` guard is gone because the inner join to
`folder_workspaces` subsumes it — a folder under no tracked repository contributes
no row rather than an invented workspace. Measured 2026-10-02: 0 of 13,724 folders
carry a null `repository_id` and 0 of 7,025 referenced_libraries rows fall out
that way, so nothing was dropped. The column stays NULLABLE, so that zero is the
current state, not a structural guarantee.

Consequence for fixtures: a folder inserted with a NULL `repository_id` and then
wired to a workspace by `UPDATE folders SET workspace_id` produces NO row here. A
fixture must create a `repositories` row, a `repositories_in_workspaces` row, and set
`folders.repository_id`.

`folder_workspaces` is referenced directly rather than restated, and NOT wrapped in
a CTE — in `graph_nodes` the identical SQL spelled as a CTE stopped the planner
parameterising the view by the consumer''s key. Plan shape for
`WHERE workspace_id = $1`, EXPLAIN at `max_parallel_workers_per_gather = 0` on
2026-10-02: Seq Scan on the 261-row `repositories_in_workspaces` -> Bitmap Index Scan
on `folders_repository_id_idx` -> Index Only Scan on `referenced_libraries_pkey`
-> Index Scan on `libraries_pkey`. No sequential scan of `folders` or
`referenced_libraries`. (No claim is made here about what the
`folders.workspace_id` form planned as: swept across all 250 workspaces it has no
single shape — a Nested Loop with Memoize in the large majority, a Hash Join in
a handful — so any one description of it would be a minority case stated as the
rule.) The `left join workspaces` inside
`folder_workspaces` is eliminated outright, because no column of it is selected
here. No wall-clock is quoted: `folders` is large enough relative to
shared_buffers that repeated identical runs are not comparable.';
