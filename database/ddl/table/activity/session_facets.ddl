set search_path to activity, sensei, extensions;

-- One LLM-derived record per session, backing the developer retrospective
-- (spec 2026-08-26-session-facets-and-retrospective-report).
--
-- The mechanical half of a retrospective lives on activity.sessions — turns,
-- corrections, tokens, duration, model. It cannot say what the person was trying
-- to do, what got in the way, or whether it worked, and those are the questions
-- a retrospective exists to answer. This table holds that reading.
--
-- Produced by the same gated pass as the process-quality judgments
-- (sessions.process_analyzed_at), not a second one: a separate pass would double
-- the per-session cost and add a second incremental gate to keep in step.
--
-- `brief_summary` is deliberately ABSENT — activity.sessions.summary already
-- holds it, and duplicating a column is how two stores of one fact diverge.
create table if not exists activity.session_facets (
    session_id       text primary key                    -- activity.sessions.client_session_id (the assistant's own id)
  , underlying_goal  text        not null                -- one sentence: what they were trying to achieve
  , goal_outcome     goal_outcome                        -- whether it landed; see the enum for why this is not session_outcome
  , primary_success  text        not null default ''     -- what went best, in a short phrase
  , friction_detail  text        not null default ''     -- one sentence; '' when the session ran clean
  , analyzed_at      timestamptz not null default now()
);

-- The retrospective groups by outcome across a person's sessions.
create index if not exists session_facets_outcome_idx
    on activity.session_facets (goal_outcome)
 where goal_outcome is not null;

comment on table activity.session_facets is
'LLM-derived retrospective record, one row per session (spec 2026-08-26).
Scalar facets only; the goal/friction multisets live in
activity.session_facet_tags and the grounding quotes in
activity.session_process_evidence. brief_summary is NOT here — it is
activity.sessions.summary.';
comment on column activity.session_facets.session_id is
'The assistant''s own session id (activity.sessions.client_session_id), matching
session_process_evidence.session_id — the join key for the cited turns. Not an FK:
a session row can be pruned while its derived record and quotes stay legible.';
comment on column activity.session_facets.underlying_goal is
'One sentence describing what the person was trying to achieve. Every row is
grounded: a record whose evidence quote could not be found verbatim in the
transcript is DROPPED by the analyzer, never stored (spec D5).';
comment on column activity.session_facets.goal_outcome is
'Whether the session achieved its goal, read from the transcript. Distinct from
activity.sessions.outcome, which is mechanical — a session is routinely both
`corrected` and `mostly_achieved`. NULL means the analyzer would not commit to a
reading.';
comment on column activity.session_facets.friction_detail is
'One sentence naming what got in the way. Empty string means the analyzer saw no
friction — distinct from a session with no record at all, which is simply absent
from this table.';
