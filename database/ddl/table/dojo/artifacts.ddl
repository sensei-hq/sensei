set search_path to dojo, extensions;

create sequence if not exists dojo.artifacts_seq;

create table if not exists dojo.artifacts (
  id             uuid                 primary key default gen_random_uuid()
, seq            bigint               not null default nextval('dojo.artifacts_seq')
, tenant_id      uuid                 not null references dojo.tenants(id)
, engagement_id  uuid                 references dojo.engagements(id)
, kind           dojo.artifact_kind   not null
, status         dojo.artifact_status not null default 'submitted'
, title          text                 not null
, body           text                 not null
, payload        jsonb                not null default '{}'
, scope          jsonb                not null default '{}'
, signature      text                 not null
, attribution    jsonb                not null default '{}'
, dereferenced   boolean              not null default false
, contributed_by uuid
, approved_by    uuid
, adopted_count  integer              not null default 0
, muted_count    integer              not null default 0
, pinned_count   integer              not null default 0
, published_at   timestamptz
, created_at     timestamptz          not null default now()
, updated_at     timestamptz          not null default now()
);

create index if not exists artifacts_seq_idx on dojo.artifacts(seq);
create index if not exists artifacts_tenant_idx on dojo.artifacts(tenant_id);
create index if not exists artifacts_signature_idx on dojo.artifacts(tenant_id, signature);
create index if not exists artifacts_kind_idx on dojo.artifacts(tenant_id, kind);
create index if not exists artifacts_engagement_idx on dojo.artifacts(engagement_id);

comment on table dojo.artifacts is
'The canonical Dōjō artifact — one of six kinds (principle/pattern/prompt/guard/
skill/agent) carried by the loop. Type-specific fields live in payload (jsonb
keyed by kind); upstream and downstream shapes are identical so the round-trip
preserves the payload. A published artifact distributes downstream to every
membership whose scope matches this artifact''s scope tag.';

comment on column dojo.artifacts.seq
     is 'Monotonic federation pull cursor, advanced on every write (the service sets seq = nextval on publish; a bigserial-style default alone would only fire on insert). A puller resumes gap-free from the last seq it saw — mirrors dojo.shared_rules.seq.';
comment on column dojo.artifacts.engagement_id
     is 'Set when the artifact came from client work under an engagement; such artifacts must have dereferenced=true.';
comment on column dojo.artifacts.kind
     is 'One of the six primitives: principle, pattern, prompt, guard, skill, agent.';
comment on column dojo.artifacts.payload
     is 'Type-specific fields keyed by kind (e.g. a skill''s manifest, an agent''s tools/scope, a guard''s check).';
comment on column dojo.artifacts.scope
     is 'Scope tag deciding who receives it downstream: {company|team|project|stack}.';
comment on column dojo.artifacts.signature
     is 'Content signature for dedup/clustering — near-identical contributions share a signature and are merged in triage.';
comment on column dojo.artifacts.attribution
     is 'Credit metadata: {author, org, dereferenced, anonymous_id}. Governed by the contributing membership''s attribution rules.';
comment on column dojo.artifacts.dereferenced
     is 'True when the source reference was stripped (mandatory for client work). The lead audit filters on this.';
comment on column dojo.artifacts.contributed_by
     is 'user_id of the contributor (or null when anonymised).';
comment on column dojo.artifacts.approved_by
     is 'user_id of the maintainer who approved publication (null while submitted or when auto-approved).';
comment on column dojo.artifacts.adopted_count
     is 'Downstream landing telemetry: consumers who applied/pinned this (rolled up from dojo.downstream_inbox).';
comment on column dojo.artifacts.muted_count
     is 'Downstream landing telemetry: consumers who muted this.';
comment on column dojo.artifacts.pinned_count
     is 'Downstream landing telemetry: consumers who pinned this.';
