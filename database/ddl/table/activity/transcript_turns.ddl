set search_path to activity, extensions;

-- Assistant + user prose per conversation turn, parsed from agent transcripts
-- (#73). The hook stream captures tool calls + prompts but NOT assistant prose;
-- this backfills that from ~/.claude transcripts (and later Zed), so the
-- analyzer's LLM tiers can mine "good-catch"/learnings + correction context.
-- Grain = one user-prompt -> assistant-response turn (aligns with
-- activity.turns). Provenance: `source` = capture origin (claude_code/zed),
-- distinct from `family` (the model). Identity = (source, session_id, turn_index).
create table if not exists activity.transcript_turns (
    id             uuid primary key default gen_random_uuid()
  , source         text not null
  , session_id     text not null
  , family         text
  , turn_index     integer not null
  , user_text      text
  , assistant_text text
  , char_count     integer not null default 0
  , started_at     timestamptz
  , created_at     timestamptz not null default now()
  , unique (source, session_id, turn_index)
);

create index if not exists transcript_turns_session_idx
  on activity.transcript_turns (session_id);
create index if not exists transcript_turns_source_session_idx
  on activity.transcript_turns (source, session_id);

comment on table activity.transcript_turns is
'Per-turn assistant/user prose parsed from agent transcripts (#73). Backfills the
prose the hook stream lacks; consumed by the analyzer LLM tiers. Grain = one
user-prompt -> assistant-response turn. Identity = (source, session_id, turn_index).';
