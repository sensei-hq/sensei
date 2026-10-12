set search_path to dojo, sensei, extensions;

create sequence if not exists dojo.shared_rules_seq;

create table if not exists dojo.shared_rules (
  id            uuid        primary key default gen_random_uuid()
, seq           bigint      not null default nextval('dojo.shared_rules_seq'::regclass)
, namespace_id  uuid        not null references sensei.namespaces(id)
, content_hash  text        not null
, rule_type     text        not null
, title         text        not null
, content       text        not null
, impact        text
, enforcement   enforcement not null
, status        text        not null default 'active'
, version       integer     not null default 1
, origin_repo   text
, published_by  text        not null
, published_at  timestamptz not null
, updated_at    timestamptz not null default now()
, tenant_id     uuid        references dojo.tenants(id) on delete cascade
, constraint shared_rules_tenant_ns_content unique (tenant_id, namespace_id, content_hash)
);

create index if not exists shared_rules_seq_idx on dojo.shared_rules(seq);

comment on table dojo.shared_rules is
'Published-rule registry for the dojo-mind. A flattened snapshot of a promoted
rule (no memory graph). seq is a monotonic cursor advanced on every insert,
republish, and tombstone (the store sets seq = nextval on every write — bigserial
alone would only fire on insert). Self-contained: no FK to projects/folders/sessions.';

-- RLS: database/policies/dojo/shared_rules.sql — it calls functions, so it is
-- applied after them (database/policies/README.md). Members read their tenant's
-- rules; maintainers and above write them.
alter table dojo.shared_rules enable row level security;

comment on column shared_rules.seq
     is '`::regclass` is not decoration. Postgres NORMALISES a sequence default to nextval(''...''::regclass) in the catalog, so a design written without the cast never matches what the database reports: `dbd diff` reports a difference forever and `reconcile` re-applies the same ALTER on every run without converging. That also makes `dbd diff --exit-code` unusable as a CI gate, which is what surfaced it.';
comment on column shared_rules.tenant_id
     is 'The tenant that OWNS the rule (decision 2026-10-12, #236). Members read their tenant''s rules; maintainers and above write them. NULL only for a rule whose owner the backfill could not prove (no publish audit event) — visible to no one until a maintainer re-publishes it. Becomes NOT NULL once no such row remains.';
