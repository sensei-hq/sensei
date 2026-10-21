set search_path to dojo, extensions;

-- Which teams work on which workspaces — a grouping for "by team" reports
-- (#307 D7). It grants no access: repository roles do (D5).
--
-- A mapping table rather than a `team_id` column on `workspaces`, so a workspace can
-- be shared by more than one team inside a tenant without duplicating it. That
-- happens in practice: a platform team and a product team both working the same
-- repository set.
create table if not exists team_workspaces (
  team_id      uuid        not null references dojo.teams(id)      on delete cascade
, workspace_id uuid        not null references dojo.workspaces(id) on delete cascade
, added_at     timestamptz not null default now()
, primary key (team_id, workspace_id)
);

create index if not exists team_workspaces_workspace_idx on team_workspaces(workspace_id);

comment on table team_workspaces is
'Which teams work on which workspaces, for reports (#307 D7); grants no access. Deliberately many-to-many: a workspace may be worked by
several teams in one tenant, and duplicating the workspace per team would fork its
metrics.';

alter table team_workspaces enable row level security;
drop policy if exists team_workspaces_service_only on team_workspaces;
create policy team_workspaces_service_only on team_workspaces
    for all to authenticated, anon using (false) with check (false);
