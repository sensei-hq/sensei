set search_path to dojo, extensions;

create table if not exists dojo.projects (
  id             uuid                          primary key default gen_random_uuid()
, user_id        uuid                          not null
, tenant_id      uuid                          references dojo.tenants(id)
, slug           text                          not null
, name           text                          not null
, classification dojo.project_classification   not null default 'personal'
, phase          dojo.project_phase            not null default 'watch'
, last_run_at    timestamptz
, runs_week      integer                       not null default 0
, constitution   jsonb
, created_at     timestamptz                   not null default now()
, updated_at     timestamptz                   not null default now()
, constraint projects_user_slug_unique unique (user_id, slug)
);

create index if not exists projects_user_idx on dojo.projects(user_id);
create index if not exists projects_tenant_idx on dojo.projects(tenant_id);

comment on table dojo.projects is
'A project the user works, as the Dōjō sees it — the source of truth for the
personal projects list, the per-dōjō project count, and the per-project
constitution drill-in. Populated by the daemon: on any project with a relay run,
the daemon upserts a row from the plan payload ({slug, name, classification,
phase}). Owner is the USER (not a membership): a personal project has no dōjō /
tenant, so it can''t key on a membership — and unlike relay_sessions there is no
re-ownership to go stale, the user is the direct owner. tenant_id binds an
org-classified project to its dōjō (null for personal). The read is user-wide
across the user''s memberships (WS-0 Rule A intent): every row the user owns.';

comment on column dojo.projects.user_id
     is 'Owner (Supabase auth subject). Not a local FK — identity is owned by Supabase (cf. dojo.memberships.user_id). RLS scopes rows to this user.';
comment on column dojo.projects.tenant_id
     is 'The dōjō this project is bound to (org-classified work); null for a personal project.';
comment on column dojo.projects.slug
     is 'The dereferenced project identity — the drill-in key (user-wide unique). Never a filesystem path or a client-identifying string; the daemon dereferences before sending.';
comment on column dojo.projects.classification
     is 'company | client | personal | community — drives the governance ladder + badge. client work is source-dereferenced on publish (always-on).';
comment on column dojo.projects.phase
     is 'watch | notice | adopt — adoption phase in the loop. Semantic order, but the enum deploys alphabetically; rank in code.';
comment on column dojo.projects.runs_week
     is 'Relay runs against this project in the trailing week (mirrored from the daemon for the projects list).';
comment on column dojo.projects.constitution
     is 'The daemon-resolved governing constitution for the per-project drill-in
(F4), as the RelayConstitution wire shape: {rules:[{level,text,hard}],
conflicts:[{topic,loser_level,winner_level,why,locked}],locks}. The daemon OWNS
resolution (dedup + mandatory locks + discards); the dōjō only displays it, never
re-resolves. Null until the daemon federates it → the dōjō shows its "resolves in
your editor" state (honest-empty, never a fabricated ladder).';

-- Row-level security — the client-direct read path connects as `authenticated`;
-- RLS narrows to the user's own projects. A table-level GRANT is still required
-- for the role to touch the table at all (without it RLS never even evaluates).
-- SELECT only — the daemon upserts via the service_role path. Mirrors the
-- relay_sessions own-rows pattern, but keyed on user_id directly (projects have
-- no membership indirection). `(select auth.uid())` is evaluated once per query.
alter table dojo.projects enable row level security;

grant select on dojo.projects to authenticated;

drop policy if exists projects_select_own on dojo.projects;
create policy projects_select_own
    on dojo.projects
    for select
    to authenticated
    using (user_id = (select auth.uid()));
