set search_path to sensei, extensions;

create table if not exists scopes (
  key                      text        primary key
, name                     text        not null
, level                    integer     not null
, shareable                boolean     not null default false
, description              text
, modified_at              timestamptz not null default now()
);

comment on table scopes is
'The governance scope ladder. Each row is one level at which rules/knowledge
and identity can apply. Precedence is the integer `level` — higher = more
specific = wins on conflict (most-specific-scope-wins), with enforcement as a
separate override axis (see governance concept doc). A repo is a member of a
SET of namespaces across these scopes (no single-parent tree); resolution
orders by level. Seeded via staging.import_scopes().';

comment on column scopes.key
     is 'Stable identifier (e.g. general, user, organization, client, technology, team, project, repository).';
comment on column scopes.name
     is 'Human-readable label.';
comment on column scopes.level
     is 'Precedence rank. Higher = more specific = wins. general=0 … repository=70.';
comment on column scopes.shareable
     is 'Whether knowledge at this scope federates to a shared hive-mind (organization/client/team = true).';
comment on column scopes.description
     is 'What this scope is for.';
comment on column scopes.modified_at
     is 'Timestamp of the last modification to this row.';
