-- A dōjō has no Clients section (#307 R7, D34, user 2026-10-22): a client is
-- just another dōjō, and a dōjō's work is its Epics, Sprints and Tasks from its
-- integrations. The engagement and incident model, and the confidentiality
-- ledger built on it, are gone from the dōjō's database:
--   dojo.engagements, dojo.incidents, their enums (engagement_status,
--   incident_severity, incident_status), the dojo.confidentiality_ledger view,
--   dojo.is_confidentiality_action, dojo.principal_is_member (only the incidents
--   policy asked it), and the engagement_id columns on audit_events/artifacts.
--
-- The release reconciles WITHOUT `--prune`, so a table that leaves the design is
-- not dropped by dbd; apply/before/dojo_clients_retired.sql drops the tables and
-- the view (and retired_functions.sql / retired_types.sql the functions and
-- enums). The reconcile drops the columns itself, with the statements it planned
-- before the hook ran — so the hook must leave those for it, and must clear what
-- would block them.
-- This test rebuilds the shape an older dōjō holds — with a row in each table —
-- runs the hooks, then the reconcile's own statements, and checks it is all
-- gone; then runs the hooks again (a no-op).
--
-- WHAT BREAKS THIS TEST: any of the objects back in the design; a hook that
-- misses one, refuses on a populated table, takes away what the reconcile plans
-- to drop (its DROP COLUMN has no IF EXISTS), or fails on its second run.

begin;
set local client_min_messages = warning;

create temp table client_objects (kind text, name text) on commit drop;
insert into client_objects values
  ('table',    'dojo.engagements'),
  ('table',    'dojo.incidents'),
  ('view',     'dojo.confidentiality_ledger'),
  ('type',     'dojo.engagement_status'),
  ('type',     'dojo.incident_severity'),
  ('type',     'dojo.incident_status'),
  ('function', 'dojo.is_confidentiality_action(text)'),
  ('function', 'dojo.principal_is_member(uuid,uuid)'),
  ('column',   'dojo.audit_events.engagement_id'),
  ('column',   'dojo.artifacts.engagement_id');

create function pg_temp.client_objects_left() returns text language sql as $$
    select coalesce(string_agg(name, ', ' order by name), '')
      from client_objects o
     where case o.kind
             when 'table'    then to_regclass(o.name) is not null
             when 'view'     then to_regclass(o.name) is not null
             when 'type'     then to_regtype(o.name) is not null
             when 'function' then to_regprocedure(o.name) is not null
             when 'column'   then exists (
                 select 1 from information_schema.columns c
                  where c.table_schema = split_part(o.name, '.', 1)
                    and c.table_name   = split_part(o.name, '.', 2)
                    and c.column_name  = split_part(o.name, '.', 3))
           end;
$$;

-- ── the deployed design has none of them ─────────────────────────────────────
do $$
begin
    if pg_temp.client_objects_left() <> '' then
        raise exception 'the dōjō still has its Clients objects: %', pg_temp.client_objects_left();
    end if;
end $$;

-- ── the shape an older dōjō holds, populated ─────────────────────────────────
create type dojo.engagement_status as enum ('active', 'ended');
create type dojo.incident_severity as enum ('low', 'medium', 'high', 'critical');
create type dojo.incident_status   as enum ('open', 'investigating', 'resolved');
create table dojo.engagements (
  id          uuid primary key default gen_random_uuid()
, tenant_id   uuid not null references dojo.tenants(id)
, client_name text not null
, status      dojo.engagement_status not null default 'active'
);
alter table dojo.audit_events add column engagement_id uuid references dojo.engagements(id);
alter table dojo.artifacts    add column engagement_id uuid references dojo.engagements(id);
create index audit_events_engagement_idx on dojo.audit_events(engagement_id);
create index artifacts_engagement_idx on dojo.artifacts(engagement_id);
create table dojo.incidents (
  id            uuid primary key default gen_random_uuid()
, tenant_id     uuid not null references dojo.tenants(id)
, engagement_id uuid references dojo.engagements(id)
, artifact_id   uuid references dojo.artifacts(id)
, title         text not null
, severity      dojo.incident_severity not null default 'medium'
, status        dojo.incident_status   not null default 'open'
, owner_id      uuid
);
alter table dojo.incidents enable row level security;
create function dojo.is_confidentiality_action(p_action text) returns boolean
    language sql immutable as $$ select p_action in ('publish', 'contained', 'held') $$;
create function dojo.principal_is_member(p_tenant uuid, p_principal uuid) returns boolean
    language sql stable as $$ select true $$;
create view dojo.confidentiality_ledger as
select id, tenant_id, ts, action, target, detail, engagement_id
  from dojo.audit_events
 where dojo.is_confidentiality_action(action);
drop policy if exists audit_events_read on dojo.audit_events;
create policy audit_events_read on dojo.audit_events for select to authenticated
    using (dojo.tenant_role_at_least(tenant_id, 'lead') and dojo.is_confidentiality_action(action));
create policy incidents_write on dojo.incidents for all to authenticated
    using (true) with check (owner_id is null or dojo.principal_is_member(tenant_id, owner_id));

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-0000000007a1', 'organization/ztest-r7', 'organization',
   'ztest-r7', 'r7 fixture', 'dojo.sensei-hq.org/organization/ztest-r7');
insert into dojo.engagements (id, tenant_id, client_name) values
  ('c0000000-0000-4000-8000-0000000007a1', 'b0000000-0000-4000-8000-0000000007a1', 'Globex');
insert into dojo.incidents (tenant_id, engagement_id, title) values
  ('b0000000-0000-4000-8000-0000000007a1', 'c0000000-0000-4000-8000-0000000007a1', 'near-leak');
insert into dojo.audit_events (tenant_id, action, target, engagement_id) values
  ('b0000000-0000-4000-8000-0000000007a1', 'publish', 'ztest-r7', 'c0000000-0000-4000-8000-0000000007a1');

-- ── the hooks, then the reconcile's own plan, take it all away ───────────────
\ir ../../apply/before/dojo_clients_retired.sql

do $$
begin
    if exists (select 1 from pg_constraint where contype = 'f' and conname like '%engagement%') then
        raise exception 'the hook left a foreign key into dojo.engagements: dbd cannot drop an unnamed one';
    end if;
    if to_regtype('dojo.engagement_status') is null
       or not exists (select 1 from information_schema.columns
                       where table_schema = 'dojo' and table_name = 'artifacts' and column_name = 'engagement_id') then
        raise exception 'the hook took a column or enum the reconcile plans to drop — its DROP would fail';
    end if;
end $$;

-- What `dbd reconcile --scope dojo --allow-destructive` executes here (its plan,
-- rehearsed on a copy of production), after the before-hooks.
alter table dojo.artifacts drop column engagement_id;
drop index if exists dojo.artifacts_engagement_idx;
alter table dojo.audit_events drop column engagement_id;
drop index if exists dojo.audit_events_engagement_idx;

\ir ../../apply/after/retired_functions.sql
\ir ../../apply/after/retired_types.sql
-- The policies step (dbd deploy) recreates the audit read policy, admin-only.
\ir ../../policies/dojo/audit_events.sql

do $$
begin
    if pg_temp.client_objects_left() <> '' then
        raise exception 'the hooks left Clients objects behind: %', pg_temp.client_objects_left();
    end if;
    if not exists (select 1 from dojo.audit_events where target = 'ztest-r7' and action = 'publish') then
        raise exception 'the hook dropped an audit row along with its engagement column';
    end if;
end $$;

-- ── a second run changes nothing ─────────────────────────────────────────────
\ir ../../apply/before/dojo_clients_retired.sql
\ir ../../apply/after/retired_functions.sql
\ir ../../apply/after/retired_types.sql

do $$
begin
    if pg_temp.client_objects_left() <> '' then
        raise exception 'a second run brought Clients objects back: %', pg_temp.client_objects_left();
    end if;
end $$;

rollback;
