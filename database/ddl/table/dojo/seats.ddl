set search_path to dojo, sensei, extensions;

-- A tenant billing seat: one row per user's ACTIVE participation on a project
-- within a tenant. This is the accountable, auditable basis for per-seat billing.
--
-- Billable ≠ "has access". GitHub org/repo membership defines who *could* work on
-- a repo, but a seat is opened only when a user is ACTIVELY using sensei on the
-- project (a federated run/session for that user+project), and billing counts
-- ONLY active seats — never everyone with repo access. The GitHub identity behind
-- a seat is reachable via user_id → dojo.identities (GitHub-linked) and the org
-- via dojo.memberships.org_slugs, so the count can be defended member-by-member.
--
-- A tenant's billable seat count = COUNT(DISTINCT user_id) over ACTIVE
-- (ended_at IS NULL) rows whose project namespace is `private`. A user on several
-- private projects is one seat (deduped on user_id); a user only on public
-- projects consumes none.
create table if not exists dojo.seats (
  id            uuid        primary key default gen_random_uuid()
, tenant_id     uuid        not null references dojo.tenants(id)      on delete cascade
, user_id       uuid        not null   -- accountable auth subject (Supabase auth.uid, GitHub-linked via dojo.identities); not a local FK
, namespace_id  uuid        not null references sensei.namespaces(id) on delete cascade   -- the project the user is a member of; its visibility decides billability
, role          text        not null default 'contributor'   -- the user's role on THIS project (per-project, not the tenant-global role)
, started_at    timestamptz not null default now()            -- when the seat opened (first active use)
, last_active_at timestamptz                                  -- last sensei activity for this user+project; drives "actively using sensei"
, ended_at      timestamptz                                   -- NULL = active seat; set (never hard-deleted) when the user is removed / leaves the project
);

-- At most one ACTIVE seat per (user, project); closed rows (ended_at set) are
-- retained as history, so re-adding a removed user opens a fresh seat.
create unique index if not exists seats_active_user_ns_idx
    on dojo.seats(user_id, namespace_id) where ended_at is null;

create index if not exists seats_tenant_idx on dojo.seats(tenant_id);
create index if not exists seats_namespace_idx on dojo.seats(namespace_id);

comment on table dojo.seats is
'A tenant billing seat — one row per user''s active participation on a project.
Billable = COUNT(DISTINCT user_id) over active (ended_at IS NULL) rows on private
projects (namespaces.visibility=private). Only users actively using sensei are
seated, NOT everyone with GitHub repo access; the GitHub identity/org behind each
seat (user_id → dojo.identities, tenant → dojo.memberships.org_slugs) makes the
count auditable and defensible. started_at/ended_at give the seat its lifecycle;
ended_at is set on removal (soft close, history retained), never a hard delete.';
comment on column dojo.seats.user_id
     is 'The accountable user (Supabase auth.uid, GitHub-linked via dojo.identities). Not a local FK — identity is owned by Supabase. Deduped across a user''s projects to one billable seat.';
comment on column dojo.seats.namespace_id
     is 'The project the user is a member of (a sensei.namespaces instance, scope=project). Its visibility (private/public) decides whether this participation is billable.';
comment on column dojo.seats.role
     is 'The user''s role on THIS project (contributor/maintainer/lead/…) — per-project, distinct from the tenant-global dojo.memberships.role.';
comment on column dojo.seats.last_active_at
     is 'Timestamp of the user''s last sensei activity on this project. Drives the "actively using sensei" distinction; a route may require recency within the billing window before counting the seat.';
comment on column dojo.seats.ended_at
     is 'NULL = active seat (counts toward billing). Set to the removal time when the user leaves the project — the row is retained for audit, never deleted.';
