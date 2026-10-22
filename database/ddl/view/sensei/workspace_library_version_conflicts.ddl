set search_path to sensei, extensions;

create or replace view workspace_library_version_conflicts as
with per_folder as (
  select fp.workspace_id
       , rl.library_id
       , rl.version_used
       , f.id           as folder_id
       , f.name         as folder_name
    from sensei.referenced_libraries rl
    join sensei.folders f          on f.id = rl.folder_id
    join sensei.folder_workspaces fp on fp.folder_id = f.id
   where rl.version_used is not null
     and rl.version_used <> ''
     -- Exclude local-protocol deps (link:/workspace:/file:/path=) so only
     -- real registry-version drift surfaces.
     and coalesce(rl.props ? 'local_source', false) = false
),
conflicts as (
  select workspace_id, library_id
    from per_folder
   group by workspace_id, library_id
  having count(distinct version_used) > 1
)
select c.workspace_id
     , c.library_id
     , l.name                                                             as library_name
     , l.ecosystem::text                                                  as ecosystem
     , array_agg(distinct pf.version_used  order by pf.version_used)      as versions
     , array_agg(distinct pf.folder_name  order by pf.folder_name)        as folders
  from conflicts c
  join per_folder pf     on pf.workspace_id = c.workspace_id and pf.library_id = c.library_id
  join sensei.libraries l on l.id = c.library_id
 group by c.workspace_id, c.library_id, l.name, l.ecosystem;

comment on view workspace_library_version_conflicts is
'Per-workspace libraries pinned to different versions across folders.
Excludes local-protocol deps (link:/workspace:/file:/path=) so only registry
version drift surfaces. Powers the Track 3 Libraries screen "version conflicts"
signal.
- versions: array of distinct version_used values for the (workspace, library)
- folders: array of distinct folder names contributing conflicting pins

Membership comes from `folder_workspaces`, never from `folders.workspace_id`. The
old column let each folder of one repository name a different workspace, which is
how the drift this replaces began; `repositories_in_workspaces` now holds membership
and `folder_workspaces` resolves it through `folders.repository_id` — a plain
join, no function scan.

What that resolution actually plans as, per EXPLAIN (ANALYZE, BUFFERS) on
2026-10-01: a Hash Join whose build side is a `Seq Scan on repositories_in_workspaces`
reading all 261 rows in 3 buffers, probed by `folders.repository_id`. That is
the correct plan for a 3-page table, and it is a seq scan, not an index
lookup — `project_repositories_repository_id_idx` exists and the planner rightly
declines it. The `left join workspaces` inside `folder_workspaces` is eliminated
outright, because no column of it is selected here.

`folders` is still joined here for `name`, which `folder_workspaces` does not
carry, so the inlined view probes `folders_pkey` a second time on the same id.
Resolving against `repositories_in_workspaces` directly was measured as the
alternative on 2026-10-01 and returns byte-identical rows (0 either way under a
symmetric EXCEPT ALL). That second probe is its own Memoize node and reports
`hit=1683 read=0`, identically on every repetition: 6,970 input rows, 6,409
memoize hits, 561 index descents. One place owning the membership rule is worth
a memoized in-memory PK lookup.

No wall-clock is quoted for this view, and no buffer-READ total that is not
structurally zero. Reads here swing with cache warmth alone — the same plan
re-run back to back moved by hundreds of blocks with every row count, every loop
count and every Memoize hit count unchanged. Row counts, loop counts, hit counts
and page counts of fixed-size tables are the stable evidence; read totals and
milliseconds are not. (The `read=0` above qualifies because it is structural:
a fully-memoized node performs no read by construction.)

MULTI-VALUED SOURCE, DELIBERATELY NOT COLLAPSED. A repository is keyed on its
remote, so one repository can serve more than one workspace (measured 2026-10-01:
of the 253 repositories that belong to any workspace, 8 serve two each — the most
any serves — among them bridge -> {bridge, sparsh} and kavach -> {kavach,
vite-multi-adapter}).
A folder of such a repository therefore enters `per_folder` once PER WORKSPACE,
and that is correct: the pin genuinely is a pin of both workspaces, and each
workspace is entitled to see its own conflict row. Taking one workspace per folder
instead would be the arbitrary pick that caused the original defect. The
expansion is visible and small: 6,970 surviving `referenced_libraries` rows
become 8,013 `per_folder` rows.

That duplication cannot inflate anything here, but NOT for the reason it is
tempting to give. A folder does NOT yield at most one `per_folder` row per
workspace: `per_folder`''s grain is (referenced_libraries row x workspace), so one
folder contributes as many rows as it has library references. Measured
2026-10-01 at `max_parallel_workers_per_gather=0`, the maximum for a single
(folder, workspace) is 124, and 576 pairs exceed one.

What makes it safe is the AGGREGATION, not the grain. Both aggregates are
set-valued (`array_agg(distinct ...)`) and the conflict test is
`count(distinct version_used)`, so a repeated (workspace, library, version)
cannot change a value. The output grain is one row per (workspace, library).

AND THE OUTPUT GREW, which is the most consequential measured fact about this
migration and belongs here rather than in a commit message: 349 rows under the
`folders.workspace_id` form, 568 under this one — a strict superset, 219 added
and none dropped. Those 219 are second-workspace copies created by the
multi-valued membership, concentrated in the workspaces served by a shared
repository. They are not duplicates: a workspace really does have that conflict
in a repository it really does own, and the old form showed it to only one of
the two owners.

The old `where f.workspace_id is not null` guard is gone because the inner join to
`folder_workspaces` subsumes it — a folder under no tracked repository contributes
no row rather than an invented workspace. Measured 2026-10-01: 0 of 13,722 folders
and 0 of 7,025 referenced_libraries rows fall out that way, because the scan
path now writes `repository_id` on the folders it creates under a tracked
repository (0 of 13,722 null today) and writes `repositories_in_workspaces` itself.
The column stays NULLABLE and the writer guards for it — a folder whose anchor
has no repository inherits none — so the zero is the current state, not a
structural guarantee.

Consequence for fixtures: a folder inserted with a NULL `repository_id` and then
wired to a workspace by `UPDATE folders SET workspace_id` produces NO row here. That
is not a regression — it is a shape the scan path can no longer write. A fixture
must create a `repositories` row, a `repositories_in_workspaces` row, and set
`folders.repository_id`.';
