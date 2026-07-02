set search_path to sensei, extensions;

-- Per-project scoping for external services (MCP servers, inference
-- providers). Mirrors extension_projects: `project_id NULL` = global scope
-- (service active in every project), `project_id = X` = only active in
-- that project. `services.installed` remains the top-level installed
-- flag; this table controls per-project enable/disable overrides once
-- installed.
create table if not exists service_projects (
  service_id   uuid         not null references sensei.services(id) on delete cascade
, project_id   uuid         references sensei.projects(id) on delete cascade
, enabled      boolean      not null default true
, props        jsonb        not null default '{}'
, modified_at  timestamptz  not null default now()
);

-- Uniqueness split by scope: one global row per service, plus at most one
-- per (service, project) pair. Partial indices mirror the extension_projects
-- pattern so the semantics stay identical between the two governance
-- surfaces.
create unique index if not exists service_projects_global_uniq
    on service_projects(service_id)
 where project_id is null;

create unique index if not exists service_projects_scoped_uniq
    on service_projects(service_id, project_id)
 where project_id is not null;

create index if not exists service_projects_project_id_idx
    on service_projects(project_id)
 where enabled and project_id is not null;

comment on table service_projects is
'Per-project scope for services (MCP servers, inference providers).
project_id NULL = global (active in every project).
project_id = X  = active only in that project.
services.installed is the top-level "is this installed" flag; this table
controls whether an installed service is enabled for a given project so
the Instruments playground can offer per-project MCP configurations.';

comment on column service_projects.service_id
     is 'FK to services — the MCP server / inference provider being scoped.';
comment on column service_projects.project_id
     is 'FK to projects — null for the global row (default for every project), otherwise the project-specific override.';
comment on column service_projects.enabled
     is 'Whether this scope activates the service. false disables the service for the given project (or globally when project_id is null).';
comment on column service_projects.props
     is 'Per-scope config overrides (JSONB) — e.g. project-specific env, endpoint, or budget.';
comment on column service_projects.modified_at
     is 'Timestamp of the last modification to this row.';
