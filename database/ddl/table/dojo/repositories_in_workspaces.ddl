set search_path to dojo, extensions;

-- Which repositories a workspace holds.
--
-- One repository belongs to exactly ONE workspace per tenant. That constraint is
-- what keeps a workspace roll-up an unambiguous sum over a disjoint set — without
-- it, a repo in two workspaces would be double-counted by every aggregate, and the
-- fix would have to be a weighting policy nobody wants to define.
--
-- Per TENANT, not globally: the same repository may sit in a client's workspace
-- and in a consultancy's own workspace, and those are different tenants with
-- different roll-ups.
create table if not exists repositories_in_workspaces (
  workspace_id  uuid not null references dojo.workspaces(id)   on delete cascade
, repository_id uuid not null references dojo.repositories(id) on delete cascade
, tenant_id     uuid not null references dojo.tenants(id)      on delete cascade
, role          text
, primary key (workspace_id, repository_id)
, unique (repository_id, tenant_id)
);

comment on table repositories_in_workspaces is
'Workspace membership for repositories. `unique (repository_id, tenant_id)` is the
decision that keeps every workspace roll-up a plain disjoint sum.';

alter table repositories_in_workspaces enable row level security;
drop policy if exists rip_service_only on repositories_in_workspaces;
create policy rip_service_only on repositories_in_workspaces
    for all to authenticated, anon using (false) with check (false);
