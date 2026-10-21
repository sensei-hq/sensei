set search_path to dojo, extensions;

-- Which teams work on which projects — a grouping for "by team" reports
-- (#307 D7). It grants no access: repository roles do (D5).
--
-- A mapping table rather than a `team_id` column on `projects`, so a project can
-- be shared by more than one team inside a tenant without duplicating it. That
-- happens in practice: a platform team and a product team both working the same
-- repository set.
create table if not exists team_projects (
  team_id     uuid        not null references dojo.teams(id)    on delete cascade
, project_id  uuid        not null references dojo.projects(id) on delete cascade
, added_at    timestamptz not null default now()
, primary key (team_id, project_id)
);

create index if not exists team_projects_project_idx on team_projects(project_id);

comment on table team_projects is
'Which teams work on which projects, for reports (#307 D7); grants no access. Deliberately many-to-many: a project may be worked by
several teams in one tenant, and duplicating the project per team would fork its
metrics.';

alter table team_projects enable row level security;
drop policy if exists team_projects_service_only on team_projects;
create policy team_projects_service_only on team_projects
    for all to authenticated, anon using (false) with check (false);
