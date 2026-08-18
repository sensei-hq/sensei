set search_path to sensei, extensions;

create table if not exists namespaces (
  id                       uuid        primary key default gen_random_uuid()
, scope_key                text        not null references sensei.scopes(key)
, name                     text        not null
, slug                     text        not null
, level                    integer
, props                    jsonb       not null default '{}'
, visibility               namespace_visibility not null default 'private'
, modified_at              timestamptz not null default now()
, constraint namespaces_unique_identity unique (scope_key, slug)
);

create index if not exists namespaces_scope_key_idx
    on namespaces(scope_key);

comment on table namespaces is
'Concrete instances of a scope — e.g. (organization, "Sensei HQ"),
(project, "sensei"), (technology, "rust"). A repo belongs to a SET of these
(via folder_namespaces); there is no parent_id tree — precedence comes from the
scope level (optionally overridden per namespace). Created at scan time from
README frontmatter (organization/project/team) and detected stack.';

comment on column namespaces.id
     is 'Surrogate primary key (UUID).';
comment on column namespaces.scope_key
     is 'Which scope this namespace instantiates (FK to scopes.key).';
comment on column namespaces.name
     is 'Display name (e.g. "Sensei HQ").';
comment on column namespaces.slug
     is 'Normalized identifier, unique within a scope (e.g. "sensei-hq").';
comment on column namespaces.level
     is 'Optional per-namespace override of scopes.level. Null = inherit from scope.';
comment on column namespaces.props
     is 'Extensible metadata (icons, source, etc.).';
comment on column namespaces.visibility
     is 'private | public. For project-scope namespaces this is the project''s visibility; a member consumes a tenant billing seat only through a `private` project. Other scopes carry the default and are ignored by billing. Defaults to private (opt into public, never accidentally out).';
comment on column namespaces.modified_at
     is 'Timestamp of the last modification to this row.';

-- Dōjō (Supabase) plane: read by the security_invoker views dojo.pack_adoption and
-- dojo.tenant_seat_usage as service_role (server-only; service_role bypasses RLS). Inert on
-- the local daemon (owner connection; never queries as service_role via PostgREST).
grant select on namespaces to service_role;
