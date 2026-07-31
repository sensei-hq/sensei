set search_path to dojo, extensions;

create table if not exists dojo.relay_sessions (
  id             uuid                  primary key default gen_random_uuid()
, membership_id  uuid                  not null references dojo.memberships(id)
, run_id         uuid                  not null
, title          text                  not null default ''
, goal           text
, status         dojo.relay_run_status not null default 'running'
, progress_done  integer               not null default 0
, progress_total integer               not null default 0
, current_phase  text
, current_feature text
, last_event_at  timestamptz
, paused_until   timestamptz
, pause_reason   text
, heartbeat_at   timestamptz
, started_at     timestamptz           not null default now()
, completed_at   timestamptz
, created_at     timestamptz           not null default now()
, updated_at     timestamptz           not null default now()
, constraint relay_sessions_membership_run_unique unique (membership_id, run_id)
);

create index if not exists relay_sessions_membership_idx on dojo.relay_sessions(membership_id, started_at desc);
create index if not exists relay_sessions_run_idx on dojo.relay_sessions(run_id);

comment on table dojo.relay_sessions is
'One row per supervised run as seen from Relay — the cloud-side presence + status
mirror of a daemon-local run (activity.runs). The daemon''s DB is separate, so
run_id is a plain uuid across the boundary (NO cross-DB FK — cf.
sensei.projects.dojo_id). status/progress are mirrored from the daemon for phone/
console display; the daemon holds the authoritative run_status. Drives the
running/paused/stuck badge and the progress rollup.';

comment on column dojo.relay_sessions.run_id
     is 'The daemon-local activity.runs(id). Plain uuid — no cross-DB FK.';
comment on column dojo.relay_sessions.status
     is 'Mirrored run status for display; authoritative value lives daemon-side.';
comment on column dojo.relay_sessions.last_event_at
     is 'Timestamp of the newest run_event — drives the "last progress N min ago" line.';
comment on column dojo.relay_sessions.heartbeat_at
     is 'Presence ping (~15–30s). A stale heartbeat flips the badge to unknown/degraded.';

-- Row-Level Security (P4.1) — client-direct Supabase Realtime read path.
-- The phone subscribes Supabase Realtime with its own JWT (role `authenticated`),
-- so it can only ever SELECT/subscribe rows RLS lets it see. The Worker keeps its
-- `service_role` write path, which BYPASSES RLS (rolbypassrls) — daemon→Worker→DB
-- writes are unaffected. SELECT-only policy: the phone only reads; every write
-- stays on the service-role path.
--
-- P4 model = own-rows-only: a user sees a run only if it is THEIR run — derived
-- from membership_id via dojo.owns_membership() (NOT a denormalized user_id copy,
-- which goes stale on re-ownership — WS-0 Rule A). Team-wide visibility (all
-- members of the run's tenant see it) is a deliberate P6 extension — NOT P4.
--
-- Idempotent: the pre-release deploy re-applies every DDL file declaratively on
-- each deploy (dbd `Current`/`Fresh` strategy re-runs ApplyEntity → re-executes
-- this file), and `create policy` is not idempotent — so drop-if-exists first.
-- `enable row level security` is idempotent (no error if already enabled).
alter table dojo.relay_sessions enable row level security;

-- The client-direct read path connects as role `authenticated`. RLS filters ROWS
-- but a table-level GRANT is still required for the role to touch the table at all
-- (without it: "permission denied for table", RLS never even evaluated). SELECT
-- only — writes stay on the service_role path. The dojo schema already grants
-- USAGE to authenticated (exposed schema); this adds the row-read privilege that
-- the own-rows policy then narrows. Idempotent (re-grant is a no-op).
grant select on dojo.relay_sessions to authenticated;

drop policy if exists relay_sessions_select_own on dojo.relay_sessions;
create policy relay_sessions_select_own
    on dojo.relay_sessions
    for select
    to authenticated
    using (dojo.owns_membership(membership_id));
