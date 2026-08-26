set search_path to dojo, extensions;

create sequence if not exists dojo.relay_inbox_seq;

create table if not exists dojo.relay_inbox (
  id            uuid                         primary key default gen_random_uuid()
-- `::regclass` is not decoration. Postgres NORMALISES a sequence default to
-- nextval('...'::regclass) in the catalog, so a design written without the cast
-- never matches what the database reports: `dbd diff` reports a difference
-- forever and `reconcile` re-applies the same ALTER on every run without
-- converging. That also makes `dbd diff --exit-code` unusable as a CI gate,
-- which is what surfaced it.
, seq           bigint                       not null default nextval('dojo.relay_inbox_seq'::regclass)
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
