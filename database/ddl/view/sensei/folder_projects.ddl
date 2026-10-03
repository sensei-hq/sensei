set search_path to sensei, extensions;

create or replace view folder_projects as
select f.id            as folder_id
     , f.abs_path      as folder_abs_path
     , f.repository_id
     , pr.project_id
     , p.name          as project
  from sensei.folders              f
  join sensei.repositories_in_projects pr on pr.repository_id = f.repository_id
  left join sensei.projects        p  on p.id = pr.project_id;

comment on view folder_projects is
'Which project(s) a folder belongs to — two indexed joins, no function.

THE ANCHOR IS RESOLVED AT WRITE TIME, NOT HERE, and that is the whole design.
`folders.repository_id` used to be set ONLY on the repo-root/checkout folder, so
a subfolder''s repository could be found only by walking ancestors through
`repo_anchor_for` — a set-returning FUNCTION. A predicate cannot be pushed into a
function scan, so `WHERE project = ''sparsh''` planned as a Seq Scan over all
13,715 folders with 13,715 function calls, to answer a question with 45 rows.

That was not a tuning detail. A migration of eight views onto the function-based
resolution was built and adversarially reviewed on 2026-09-30, and four failed
outright on this one cause: `structure_edges` 81 ms -> ~37 s for a single project
(~450x), `task_health` ~70x on the query its own comment advertises, and
`project_metrics` acquiring a LEFT JOIN that made it non-auto-updatable and
silently broke a production WRITE path.

Every folder now carries its own `repository_id`, inherited from its anchor by
`upsert_subfolder_kind` / `upsert_folder` — one anchor lookup per folder WRITTEN
rather than per row READ. A materialised view was considered and rejected: it
would have needed a refresh hook, carried staleness a reader has to be told
about, and — if it had materialised folder->project rather than folder->repository
— made a USER EDITING A PROJECT wait for the next scan to see any effect.
Populating the column that already existed is simpler than all of it.

`kind` still says which folder IS the anchor (`git` / `standalone` / `subtree`),
so no flag was needed to preserve what the sparseness used to encode.

MULTI-VALUED BY DESIGN. A canonical repository is keyed on its REMOTE, so two
checkouts collapse to one repository row, and four repositories serve two
projects each — `kavach` in `kavach` and `vite-multi-adapter`, `bridge` in
`bridge` and `sparsh`. Those folders return TWO rows, because their nodes
genuinely belong to both. A consumer needing exactly one must name which; an
arbitrary pick that looks authoritative is how the drift this replaces began.

A folder under no tracked repository returns NO row — the join finds nothing
rather than inventing a project.

Common queries:
  -- one folder’s project(s)
  SELECT project FROM folder_projects WHERE folder_id = $1
  -- every folder of a project — the replacement for `WHERE folders.project_id = $1`
  SELECT folder_id FROM folder_projects WHERE project = ''sensei''
  -- the folders whose repository serves more than one project
  SELECT folder_abs_path, count(*) FROM folder_projects GROUP BY 1 HAVING count(*) > 1';

comment on column folder_projects.repository_id is
'Carried on every folder now, inherited from its anchor at write time. `kind` still marks which folder IS the anchor, so nothing the old sparseness encoded was lost.';
comment on column folder_projects.project_id is
'From `repositories_in_projects`, never from `folders.project_id`. A folder does not carry its own project; that is what made the client-q drift possible.';
