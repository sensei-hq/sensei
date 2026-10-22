set search_path to sensei, extensions;

CREATE OR REPLACE VIEW sensei.workspace_drift AS
SELECT di.*, fp.workspace_id
  FROM inference.drift_items di
  JOIN sensei.folder_workspaces fp ON fp.folder_id = di.folder_id;

comment on view workspace_drift is
'Drift items attributed to a workspace through `folder_workspaces`, not through the
retired `folders.workspace_id` (#211).

MULTI-VALUED, inherited from `folder_workspaces`: a repository is keyed on its
remote, 8 repositories serve two workspaces each, so a drift item in one of their
folders appears ONCE PER WORKSPACE. `repositories_in_workspaces` is keyed
(workspace_id, repository_id), so a given (drift item, workspace) pair still appears
exactly once — scope the view before aggregating and nothing double-counts:

  SELECT count(*) FROM workspace_drift WHERE workspace_id = $1   -- safe
  SELECT count(*) FROM workspace_drift                       -- counts pairs, not items

A folder under no tracked repository yields no row, same as the old
`WHERE folders.workspace_id IS NOT NULL` — the join finds nothing rather than
inventing a workspace.';
