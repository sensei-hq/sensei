set search_path to sensei, extensions;

CREATE OR REPLACE VIEW sensei.project_drift AS
SELECT di.*, fp.project_id
  FROM inference.drift_items di
  JOIN sensei.folder_projects fp ON fp.folder_id = di.folder_id;

comment on view project_drift is
'Drift items attributed to a project through `folder_projects`, not through the
retired `folders.project_id` (#211).

MULTI-VALUED, inherited from `folder_projects`: a repository is keyed on its
remote, 8 repositories serve two projects each, so a drift item in one of their
folders appears ONCE PER PROJECT. `repositories_in_projects` is keyed
(project_id, repository_id), so a given (drift item, project) pair still appears
exactly once — scope the view before aggregating and nothing double-counts:

  SELECT count(*) FROM project_drift WHERE project_id = $1   -- safe
  SELECT count(*) FROM project_drift                         -- counts pairs, not items

A folder under no tracked repository yields no row, same as the old
`WHERE folders.project_id IS NOT NULL` — the join finds nothing rather than
inventing a project.';
