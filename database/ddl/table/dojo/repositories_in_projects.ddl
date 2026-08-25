set search_path to dojo, extensions;

-- Which repositories a project holds.
--
-- One repository belongs to exactly ONE project per tenant. That constraint is
-- what keeps a project roll-up an unambiguous sum over a disjoint set — without
-- it, a repo in two projects would be double-counted by every aggregate, and the
-- fix would have to be a weighting policy nobody wants to define.
--
-- Per TENANT, not globally: the same repository may sit in a client's project
-- and in a consultancy's own project, and those are different tenants with
-- different roll-ups.
create table if not exists repositories_in_projects (
  project_id    uuid not null references dojo.projects(id)     on delete cascade
, repository_id uuid not null references dojo.repositories(id) on delete cascade
, tenant_id     uuid not null references dojo.tenants(id)      on delete cascade
, role          text
, primary key (project_id, repository_id)
, unique (repository_id, tenant_id)
);

comment on table repositories_in_projects is
'Project membership for repositories. `unique (repository_id, tenant_id)` is the
decision that keeps every project roll-up a plain disjoint sum.';

alter table repositories_in_projects enable row level security;
drop policy if exists rip_service_only on repositories_in_projects;
create policy rip_service_only on repositories_in_projects
    for all to authenticated, anon using (false) with check (false);
