set search_path to sensei, extensions;

-- Per-workspace scoping for external services (MCP servers, inference
-- providers). Mirrors extension_workspaces: `workspace_id NULL` = global scope
-- (service active in every workspace), `workspace_id = X` = only active in
-- that workspace. `services.installed` remains the top-level installed
-- flag; this table controls per-workspace enable/disable overrides once
-- installed.
create table if not exists service_workspaces (
  service_id   uuid         not null references sensei.services(id) on delete cascade
, workspace_id uuid         references sensei.workspaces(id) on delete cascade
, enabled      boolean      not null default true
, props        jsonb        not null default '{}'
, modified_at  timestamptz  not null default now()
);

-- Uniqueness split by scope: one global row per service, plus at most one
-- per (service, workspace) pair. Partial indices mirror the extension_workspaces
-- pattern so the semantics stay identical between the two governance
-- surfaces.
create unique index if not exists service_workspaces_global_uniq
    on service_workspaces(service_id)
 where workspace_id is null;

create unique index if not exists service_workspaces_scoped_uniq
    on service_workspaces(service_id, workspace_id)
 where workspace_id is not null;

create index if not exists service_workspaces_workspace_id_idx
    on service_workspaces(workspace_id)
 where enabled and workspace_id is not null;

comment on table service_workspaces is
'Per-workspace scope for services (MCP servers, inference providers).
workspace_id NULL = global (active in every workspace).
workspace_id = X  = active only in that workspace.
services.installed is the top-level "is this installed" flag; this table
controls whether an installed service is enabled for a given workspace so
the Instruments playground can offer per-workspace MCP configurations.';

comment on column service_workspaces.service_id
     is 'FK to services — the MCP server / inference provider being scoped.';
comment on column service_workspaces.workspace_id
     is 'FK to workspaces — null for the global row (default for every workspace), otherwise the workspace-specific override.';
comment on column service_workspaces.enabled
     is 'Whether this scope activates the service. false disables the service for the given workspace (or globally when workspace_id is null).';
comment on column service_workspaces.props
     is 'Per-scope config overrides (JSONB) — e.g. workspace-specific env, endpoint, or budget.';
comment on column service_workspaces.modified_at
     is 'Timestamp of the last modification to this row.';
