set search_path to dojo, extensions;

create table if not exists dojo.workspaces (
  id             uuid                          primary key default gen_random_uuid()
, tenant_id      uuid                          not null references dojo.tenants(id) on delete cascade
, slug           text                          not null
, name           text                          not null
, classification dojo.workspace_classification   not null default 'personal'
, phase          dojo.workspace_phase            not null default 'watch'
, last_run_at    timestamptz
, runs_week      integer                       not null default 0
, constitution   jsonb
, created_at     timestamptz                   not null default now()
, updated_at     timestamptz                   not null default now()
, constraint workspaces_tenant_slug_unique unique (tenant_id, slug)
);

create index if not exists workspaces_tenant_idx on dojo.workspaces(tenant_id);

comment on table dojo.workspaces is
'A workspace — ONE SHARED item, the same as the sensei workspace, living in one dōjō:
an organisation''s, or the personal dōjō of the owner''s own GitHub account
(#299). It has no owner. Whoever may read one of its repositories
(dojo.repositories_in_workspaces + dojo.can_read_repository) may read it. Written
only by security-definer functions: the sync files it
(dojo.file_repository_workspace, the only creator); a relay run — a person''s
sessions IN a workspace — attaches to it (dojo.attach_run_to_workspace), moving only
last_run_at and the constitution.';

comment on column dojo.workspaces.tenant_id
     is 'The dōjō the workspace lives in: an organisation''s, or a personal dōjō for workspaces under the owner''s GitHub account. Never null.';
comment on column dojo.workspaces.slug
     is 'The workspace identity within its dōjō (unique per tenant) — the sensei workspace-namespace slug. Never a filesystem path or a client-identifying string.';
comment on column dojo.workspaces.classification
     is 'company | client | personal | community — drives the governance ladder + badge. client work is source-dereferenced on publish (always-on).';
comment on column dojo.workspaces.phase
     is 'watch | notice | adopt — adoption phase in the loop. Semantic order, but the enum deploys alphabetically; rank in code.';
comment on column dojo.workspaces.runs_week
     is 'Relay runs against this workspace in the trailing week (mirrored from the daemon for the workspaces list).';
comment on column dojo.workspaces.constitution
     is 'The daemon-resolved governing constitution for the per-workspace drill-in
(F4), as the RelayConstitution wire shape: {rules:[{level,text,hard}],
conflicts:[{topic,loser_level,winner_level,why,locked}],locks}. The daemon OWNS
resolution (dedup + mandatory locks + discards); the dōjō only displays it, never
re-resolves. Null until the daemon federates it → the dōjō shows its "resolves in
your editor" state (honest-empty, never a fabricated ladder).';

-- Row-level security lives in policies/dojo/workspaces.sql, not here: the policy
-- calls dojo.current_principal_id(), and `dbd apply` creates every table before
-- any function, so inline it would fail the deploy. See policies/README.md.
