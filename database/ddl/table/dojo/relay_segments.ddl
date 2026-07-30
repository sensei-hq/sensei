set search_path to dojo, extensions;

create table if not exists dojo.relay_segments (
  id               uuid               primary key default gen_random_uuid()
, session_id       uuid               not null references dojo.relay_sessions(id) on delete cascade
, parent_id        uuid               references dojo.relay_segments(id) on delete cascade
, seq              integer            not null default 0
, title            text               not null default ''
, summary          text
, detail           text
, state            dojo.segment_state not null default 'pending'
-- Plan-authored task metadata (labels only — zero-knowledge D10). Set when a run
-- is seeded from a registered plan (register_plan); NULL for cadence-derived and
-- TodoWrite segments.
, agent            text
, model            text
, spec_ref         text
, is_gate          boolean            not null default false
, gate_severity    dojo.gate_severity
, response_verdict text
, response_note    text
, submitted_at     timestamptz
, created_at       timestamptz        not null default now()
, updated_at       timestamptz        not null default now()
, constraint relay_segments_session_seq_unique unique (session_id, seq)
);

create index if not exists relay_segments_session_idx on dojo.relay_segments(session_id, seq);
create index if not exists relay_segments_parent_idx on dojo.relay_segments(parent_id) where parent_id is not null;
create index if not exists relay_segments_gate_idx on dojo.relay_segments(session_id) where is_gate;

comment on table dojo.relay_segments is
'The phone-renderable execution outline: Execution → Segment → Item. A top-level
segment is a Phase; a nested one (parent_id set) is a Step. Each is the unit of
review + response (PR-review batch-send). summary is the one-line, diff-free feed
text (rolled up from capture and summarized locally); detail is the expandable body.
A segment with is_gate marks a HITL point whose gate_severity is blocking (halts)
or advisory (async review, never halts).';

comment on column dojo.relay_segments.parent_id
     is 'Self-FK: null = a Phase (top-level); set = a Step within a Phase.';
comment on column dojo.relay_segments.summary
     is 'One-line, diff-free phone status — summarized locally (no code, no tool logs).';
comment on column dojo.relay_segments.response_verdict
     is 'Per-segment review verdict: approve | request_changes | comment. Null until reviewed.';
comment on column dojo.relay_segments.submitted_at
     is 'When the batched (PR-review-style) response was sent; null while a local draft.';

-- Row-Level Security (P4.1) — see the note on dojo.relay_sessions. relay_segments
-- carries no user_id, so ownership is derived by joining through the owning session:
-- a user reads/subscribes a segment only if it belongs to THEIR run. The Worker's
-- service_role writes bypass RLS. SELECT-only; team-wide visibility is P6.
-- Idempotent (drop-if-exists) because the deploy re-applies this file each time.
alter table dojo.relay_segments enable row level security;

-- Table-level SELECT grant for the `authenticated` read path (RLS filters rows;
-- the grant lets the role touch the table). See the note on dojo.relay_sessions.
grant select on dojo.relay_segments to authenticated;

drop policy if exists relay_segments_select_own on dojo.relay_segments;
create policy relay_segments_select_own
    on dojo.relay_segments
    for select
    to authenticated
    using (
        exists (
            select 1
            from dojo.relay_sessions s
            where s.id = relay_segments.session_id
              and dojo.owns_membership(s.membership_id)
        )
    );
