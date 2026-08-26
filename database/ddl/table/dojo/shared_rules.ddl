set search_path to dojo, sensei, extensions;

create sequence if not exists dojo.shared_rules_seq;

create table if not exists dojo.shared_rules (
  id            uuid        primary key default gen_random_uuid()
-- `::regclass` is not decoration. Postgres NORMALISES a sequence default to
-- nextval('...'::regclass) in the catalog, so a design written without the cast
-- never matches what the database reports: `dbd diff` reports a difference
-- forever and `reconcile` re-applies the same ALTER on every run without
-- converging. That also makes `dbd diff --exit-code` unusable as a CI gate,
-- which is what surfaced it.
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
, constraint shared_rules_ns_content unique (namespace_id, content_hash)
);

create index if not exists shared_rules_seq_idx on dojo.shared_rules(seq);

comment on table dojo.shared_rules is
'Published-rule registry for the dojo-mind. A flattened snapshot of a promoted
rule (no memory graph). seq is a monotonic cursor advanced on every insert,
republish, and tombstone (the store sets seq = nextval on every write — bigserial
alone would only fire on insert). Self-contained: no FK to projects/folders/sessions.';

-- RLS deny-by-default: server-only table (the dōjō Worker uses service_role, which
-- bypasses RLS; no client authenticated/anon grant). Locks out any direct PostgREST
-- access while service_role keeps full access; clears the Supabase rls_disabled advisor.
alter table dojo.shared_rules enable row level security;

-- No client access: authenticated/anon are denied all rows + writes; the dōjō Worker
-- reads/writes as service_role, which bypasses RLS. Explicit deny-all so RLS has a
-- policy (clears the rls_enabled_no_policy advisor). Idempotent.
drop policy if exists shared_rules_service_only on dojo.shared_rules;
create policy shared_rules_service_only on dojo.shared_rules
    for all to authenticated, anon
    using (false) with check (false);
