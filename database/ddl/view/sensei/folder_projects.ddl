set search_path to sensei, extensions;

create or replace view folder_projects as
select f.id                as folder_id
     , f.abs_path          as folder_abs_path
     , anchor.id           as repo_folder_id
     , anchor.repository_id
     , pr.project_id
     , p.name              as project
  from sensei.folders f
  join lateral (
         select a.repo_folder_id
           from sensei.repo_anchor_for(f.abs_path) a
          limit 1
       ) ra on true
  join sensei.folders              anchor on anchor.id = ra.repo_folder_id
  join sensei.project_repositories pr     on pr.repository_id = anchor.repository_id
  left join sensei.projects        p      on p.id = pr.project_id;

comment on view folder_projects is
'Which project(s) a folder belongs to — resolved, never stored.

THE MIGRATION TARGET FOR `folders.project_id`. That column was settable
independently on all 13,697 folders, so a repository''s folders could disagree
about their project, and one did: the swarco `documentation` checkout''s root sat
in project `swarco` while its 362 subfolders sat in `documentation`. Membership
now lives once, in `project_repositories`, keyed on the repository — see that
table for the decision that supersedes the folder-grain junction of D1/D2/D10.

THE RESOLUTION PATH, and every hop is load-bearing:
  folder → `repo_anchor_for` → the ANCHOR folder → its `repository_id` → the
  junction → project.
`repository_id` is set ONLY on the repo-root/checkout folder (I16), which is why
the anchor hop exists rather than reading the column off the folder itself.

MULTI-VALUED, DELIBERATELY. A canonical repository is keyed on its REMOTE, so two
checkouts collapse to one repository row, and four repositories measured
2026-09-30 serve two projects each — `kavach` in `kavach` and
`vite-multi-adapter`, `bridge` in `bridge` and `sparsh`. Those folders return TWO
rows here, because their nodes genuinely belong to both projects. A consumer that
needs exactly one must name which; being handed an arbitrary pick that looks
authoritative is how the drift this replaces got started.

A folder under no tracked repository returns NO row. `repo_anchor_for` never
fabricates an anchor, so an unattached folder has no project rather than a
guessed one.

Common queries:
  -- one folder’s project(s)
  SELECT project FROM folder_projects WHERE folder_id = $1
  -- every folder of a project, the replacement for `WHERE folders.project_id = $1`
  SELECT folder_id FROM folder_projects WHERE project = ''sensei''
  -- the folders whose repository serves more than one project
  SELECT folder_abs_path, count(*) FROM folder_projects GROUP BY 1 HAVING count(*) > 1';

comment on column folder_projects.repo_folder_id is
'The ANCHOR — the repo-root/checkout folder this one resolves through. Null is impossible here: a folder with no anchor produces no row at all.';
comment on column folder_projects.project_id is
'From `project_repositories`, not from `folders.project_id`. A folder never carries its own project; that is what made the drift possible.';
