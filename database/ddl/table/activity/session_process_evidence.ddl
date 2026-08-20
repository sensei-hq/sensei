set search_path to activity, extensions;

-- Evidence quotes grounding the LLM process-quality judgments (spec
-- 2026-08-20-transcript-process-quality-analyzer). One row per quote the model
-- relied on for a signal, referencing the exact transcript turn by
-- (session_id, turn_index). Deliberately NOT a foreign key to transcript_turns:
-- turns can be pruned by the activity pruner, but the verbatim `quote` keeps the
-- drill-down ("referenceable transcript statements") legible after the turn is
-- gone. A judgment the model cannot ground in a quote is dropped, never stored
-- (spec D5), so every row here corresponds to a real, cited turn.
create table if not exists activity.session_process_evidence (
    id          uuid primary key default gen_random_uuid()
  , session_id  text not null          -- activity.sessions.client_session_id (the assistant's own id)
  , signal      text not null          -- spec_depth | spec_deviation | refuted_findings | incomplete_analysis_llm
  , turn_index  integer not null       -- activity.transcript_turns.turn_index for this session
  , quote       text not null          -- the verbatim snippet the judgment rests on (survives turn pruning)
  , kind        text                   -- optional role of the quote within a pair: plan | action | assertion | retraction
  , created_at  timestamptz not null default now()
);

-- The analyzer rewrites a session's evidence in place on re-score, so reads +
-- the delete-before-insert both filter on session_id; the signal filter powers
-- the per-signal drill-down.
create index if not exists session_process_evidence_session_idx
  on activity.session_process_evidence (session_id);
create index if not exists session_process_evidence_session_signal_idx
  on activity.session_process_evidence (session_id, signal);

comment on table activity.session_process_evidence is
'Evidence quotes grounding the LLM process-quality judgments (spec 2026-08-20).
One row per cited transcript turn per signal. Not FK-linked to transcript_turns
(turns may be pruned) — the verbatim quote keeps the drill-down legible. Every
row corresponds to a real cited turn: ungrounded judgments are dropped, not
stored (spec D5).';
comment on column activity.session_process_evidence.session_id is
'The assistant''s own session id (activity.sessions.client_session_id), matching
transcript_turns.session_id — the join key for the cited turn.';
comment on column activity.session_process_evidence.signal is
'Which judgment this quote grounds: spec_depth | spec_deviation | refuted_findings
| incomplete_analysis_llm.';
comment on column activity.session_process_evidence.kind is
'Optional role of the quote within an evidence pair: plan/action (deviation) or
assertion/retraction (refuted_findings). NULL for single-quote signals.';
