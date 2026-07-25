set search_path to activity, sensei, extensions;

create table if not exists runs (
  id              uuid        primary key default gen_random_uuid()
, project_id      uuid        references sensei.projects(id) on delete set null
, plan_ref        text        not null default ''
, goal            text
, author_name     text
, author_email    text
, status          run_status  not null default 'running'
, paused_until    timestamptz
, pause_reason    text
, current_phase   text
, current_feature text
, dojo_session_id uuid
, max_concurrency integer     not null default 1
, started_at      timestamptz not null default now()
, completed_at    timestamptz
, heartbeat_at    timestamptz
, recovery_attempts integer   not null default 0
, created_at      timestamptz not null default now()
, updated_at      timestamptz not null default now()
);

create index if not exists runs_project_idx on runs(project_id, started_at desc);
create index if not exists runs_active_idx on runs(status) where status in ('running', 'paused', 'stalled', 'blocked');

comment on table runs is
'A daemon-owned autonomous run of a multi-phase plan — the relay engine''s executor.
Survives daemon restarts (durable run-state; fixes the vacation run''s
session-coupled death). plan_ref points at the committed plan doc. status is
authoritative; the cloud dojo.relay_sessions mirrors it. paused_until + pause_reason
carry limit pauses (rate limit / weekly cap) with auto-resume. dojo_session_id is
the cloud relay session''s id (plain uuid — no cross-DB FK). max_concurrency is
throttled to 1 under rate-limit pressure.';

comment on column runs.plan_ref
     is 'Path/ref of the committed plan doc the run executes.';
comment on column runs.author_name
     is 'Git author name (user.name, local→global) resolved for the run''s project at start_run — who is doing the work. Matches the commit author + the Dōjō sign-in. NULL when git identity was unresolvable.';
comment on column runs.author_email
     is 'Git author email (user.email, local→global) — the identity key. Used to attribute the run to a person; the Dōjō membership resolves it to a login.';
comment on column runs.paused_until
     is 'When a limit pause auto-resumes; NULL while running. Reset time parsed from the CLI limit message.';
comment on column runs.pause_reason
     is 'Human-readable pause cause (e.g. "rate limit", "weekly cap") shown in the feed.';
comment on column runs.dojo_session_id
     is 'The cloud dojo.relay_sessions(id) mirroring this run. Plain uuid — no cross-DB FK.';
comment on column runs.max_concurrency
     is 'Active sub-agent cap; graduated response throttles this to 1 under rate-limit pressure.';
comment on column runs.recovery_attempts
     is 'Bounded auto-recovery counter — incremented on each watchdog recovery; reset to 0 on real progress (a clean drive step); at the cap the run escalates to crashed.';
