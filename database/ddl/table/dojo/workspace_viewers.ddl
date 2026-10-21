set search_path to dojo, extensions;

-- The workspace viewer (#307 D8) — the only permission a workspace has: a read-only
-- managerial view of its metrics, dashboards and reports, aggregated or split by
-- repository, never per-person. Granted by the dōjō's admins to someone in the
-- dōjō — a member, or a viewer invited in (dojo.dojo_viewers). Access to code and to
-- per-person data lives on repositories (D5, D6), not here. Written only through
-- dojo.grant_workspace_viewer / dojo.revoke_workspace_viewer.
create table if not exists workspace_viewers (
  workspace_id uuid        not null references dojo.workspaces(id) on delete cascade
, principal_id uuid        not null references dojo.principals(id) on delete cascade
, granted_by   uuid        references dojo.principals(id) on delete set null
, granted_at   timestamptz not null default now()
, primary key (workspace_id, principal_id)
);

create index if not exists workspace_viewers_principal_idx on workspace_viewers(principal_id);

comment on table workspace_viewers is
'Read-only managerial viewers of a workspace (#307 D8): its metrics, dashboards and
reports, never per-person. Admin-granted to a member or dōjō viewer.';
