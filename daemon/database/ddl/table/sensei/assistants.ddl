set search_path to sensei, extensions;

create table if not exists assistants (
  id                       uuid              primary key default gen_random_uuid()
, family                   assistant_family  not null
, name                     text              not null
, version                  text
, path                     text
, transport                text              not null default 'stdio'
, configured               boolean           not null default false
, configured_at            timestamptz
, configured_version       text
, props                    jsonb             not null default '{}'
, modified_at              timestamptz       not null default now()
, unique(family, name)
);

create index if not exists assistants_family_idx
    on assistants(family);

create index if not exists assistants_configured_idx
    on assistants(configured)
 where configured = true;

comment on table assistants is
'Detected AI coding assistants (ACPs) and their configuration state.
Bootstrap detects installed assistants and upserts rows here.
Setup wizard (step 3) registers sensei plugins/skills/commands per assistant.
On upgrade, bootstrap compares configured_version against the running sensei
version — if stale, the upgrade flow re-pushes skills, agents, hooks, and
commands to each configured assistant.

- family: groups variants of the same product (e.g. claude covers claude-code CLI and claude-code desktop)
- transport: stdio or http — how sensei communicates with this assistant
- configured: true once sensei has registered its extensions with this assistant
- configured_at: when registration last ran (null = never configured)
- configured_version: sensei version at last configuration — drives upgrade reconfiguration
- props: extra metadata — {detected_at, install_method, registration_log, capabilities}';

comment on column assistants.id
     is 'Surrogate primary key (UUID).';
comment on column assistants.family
     is 'Assistant product family: claude, cursor, zed, continue, codex, aider, opencode, kiro.';
comment on column assistants.name
     is 'Specific variant or display name (e.g. "Claude Code CLI", "Cursor").';
comment on column assistants.version
     is 'Detected version of the assistant binary/app.';
comment on column assistants.path
     is 'Filesystem path to the assistant binary or config directory.';
comment on column assistants.transport
     is 'Communication transport: stdio (pipes) or http (localhost).';
comment on column assistants.configured
     is 'True once sensei has registered its extensions (skills, commands, hooks, MCP) with this assistant.';
comment on column assistants.configured_at
     is 'Timestamp of the last configuration/registration run. Null if never configured.';
comment on column assistants.configured_version
     is 'Sensei version string at last configuration. Bootstrap compares this to running version to trigger upgrade reconfiguration.';
comment on column assistants.props
     is 'Extra metadata: {detected_at, install_method, registration_log, capabilities, supported_hooks, last_health_check}.';
comment on column assistants.modified_at
     is 'Timestamp of the last modification to this row.';
