set search_path to dojo, extensions;

create sequence if not exists dojo.relay_inbox_seq;

create table if not exists dojo.relay_inbox (
  id            uuid                         primary key default gen_random_uuid()
, seq           bigint                       not null default nextval('dojo.relay_inbox_seq')
, session_id    uuid                         not null references dojo.relay_sessions(id) on delete cascade
, segment_id    uuid                         references dojo.relay_segments(id) on delete cascade
, membership_id uuid                         not null references dojo.memberships(id)
, kind          dojo.relay_inbox_kind        not null
, direction     dojo.relay_message_direction not null
, status        dojo.relay_inbox_status      not null default 'pending'
, payload       jsonb                        not null default '{}'
, reply         jsonb
, created_at    timestamptz                  not null default now()
, answered_at   timestamptz
);

create index if not exists relay_inbox_seq_idx on dojo.relay_inbox(seq);
create index if not exists relay_inbox_session_idx on dojo.relay_inbox(session_id, created_at desc);
create index if not exists relay_inbox_membership_pending_idx on dojo.relay_inbox(membership_id) where status = 'pending';
create index if not exists relay_inbox_segment_idx on dojo.relay_inbox(segment_id) where segment_id is not null;

comment on table dojo.relay_inbox is
'Durable live-interaction rows — approvals / decisions / chat / nudges / stalls —
that survive a closed app (a push deep-links to one). Realtime broadcasts inserts;
push notifies when away. payload carries the stripped prompt + the rokkit form
schema (options etc.) — never code or diffs; reply carries the human''s response,
which the daemon consumes over /v1 to continue the held run. segment_id links a gate
to its outline segment. Distinct from dojo.notifications (passive read-later badges):
this is a two-way gate with a reply channel.';

comment on column dojo.relay_inbox.seq
     is 'Monotonic poll cursor, advanced on every write — the Worker sets seq =
nextval on insert AND on the pending→answered update (a default alone only fires on
insert). poll_inbox resumes gap-free from the last seq it saw; mirrors dojo.artifacts.seq.';
comment on column dojo.relay_inbox.payload
     is 'Stripped prompt + rokkit form schema (options/fields). No code, no diffs.';
comment on column dojo.relay_inbox.reply
     is 'The human''s response (verdict / chosen option / free-text). Consumed by the daemon.';
comment on column dojo.relay_inbox.segment_id
     is 'The outline segment this row gates/annotates (null for a free-standing chat/nudge).';

-- Row-Level Security (P4.1) — see the note on dojo.relay_sessions. Own-rows-only:
-- a user reads/subscribes only their own inbox rows — ownership derived from
-- membership_id via dojo.owns_membership() (no stale user_id copy — WS-0 Rule A).
-- The Worker's service_role writes bypass RLS. SELECT-only; team-wide visibility is P6.
-- Idempotent (drop-if-exists) because the deploy re-applies this file each time.
alter table dojo.relay_inbox enable row level security;

-- Table-level SELECT grant for the `authenticated` read path (RLS filters rows;
-- the grant lets the role touch the table). See the note on dojo.relay_sessions.
grant select on dojo.relay_inbox to authenticated;

drop policy if exists relay_inbox_select_own on dojo.relay_inbox;
create policy relay_inbox_select_own
    on dojo.relay_inbox
    for select
    to authenticated
    using (dojo.owns_membership(membership_id));
