set search_path to activity, sensei, extensions;
create table if not exists turns (
  id            uuid        primary key default gen_random_uuid()
, session_id    uuid        not null references activity.sessions(id) on delete cascade
, turn_number   integer     not null
, segment       integer     not null default 1
, started_at    timestamptz not null
, ended_at      timestamptz not null
, duration      interval
, is_correction boolean     not null default false
, triage_signal text
, tool_calls    integer     not null default 0
, unique (session_id, turn_number)
);

create index if not exists turns_session_id_idx
    on turns(session_id, turn_number);

create index if not exists turns_segment_idx
    on turns(session_id, segment);

comment on table turns is
'Per-turn breakdown of an assistant session, derived from the hook-event stream
(analyzer L0, #66). A turn spans one UserPromptSubmit to the next. `segment`
marks sub-sessions: it increments when the idle gap before a turn exceeds the
threshold, so a multi-day resumed session splits into work segments without
breaking the one-row-per-session model. Sub-sessions are a GROUP BY segment.';

comment on column turns.id            is 'Surrogate primary key (UUID).';
comment on column turns.session_id    is 'Foreign key to sessions — which session this turn belongs to.';
comment on column turns.turn_number   is '1-based turn index within the session.';
comment on column turns.segment       is 'Sub-session marker: +1 after an idle gap > threshold. Group by this for work-session boundaries.';
comment on column turns.started_at    is 'First event of the turn (the UserPromptSubmit).';
comment on column turns.ended_at      is 'Last event of the turn (before the next prompt or session end).';
comment on column turns.duration      is 'Active time within the turn (gap-aware).';
comment on column turns.is_correction is 'True if this turn''s prompt corrected the previous turn (FTR detractor).';
comment on column turns.triage_signal is 'Which correction heuristic matched (revert/correction/actually/...). Null if not a correction.';
comment on column turns.tool_calls    is 'Number of tool invocations (PostToolUse) in the turn.';
