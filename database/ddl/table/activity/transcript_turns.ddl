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
  , family         text not null
  , provider       text
  , model          text
  , turn_index     integer not null
  , user_text      text
  , assistant_text text
  , char_count     integer not null default 0
  , started_at     timestamptz
  , created_at     timestamptz not null default now()
  , attrs          jsonb not null default '{}'::jsonb
  , tokens_in      bigint   -- fresh input only (`input_tokens`)
  , tokens_out     bigint
  , cache_read     bigint   -- `cache_read_input_tokens`
  , cache_write    bigint   -- `cache_creation_input_tokens`
  , stop_reason    text
  -- Subagent work is merged into the main thread today, so its cost is invisible.
  , is_sidechain   boolean
  , skill          text
  , plugin         text
  , git_branch     text     -- per-turn branch (folders.branch is checkout-grain)
  , effort         text     -- reasoning effort requested
  , service_tier   text     -- billing tier
  , unique (source, session_id, turn_index)
);

create index if not exists transcript_turns_session_idx
  on activity.transcript_turns (session_id);
create index if not exists transcript_turns_source_session_idx
  on activity.transcript_turns (source, session_id);

create index if not exists transcript_turns_skill_idx
  on activity.transcript_turns (skill) where skill is not null;

comment on table activity.transcript_turns is
'Per-turn assistant/user prose parsed from agent transcripts (#73). Backfills the
prose the hook stream lacks; consumed by the analyzer LLM tiers. Grain = one
user-prompt -> assistant-response turn. Identity = (source, session_id, turn_index).';
comment on column transcript_turns.family
     is 'NOT NULL: `TranscriptAdapter::family()` returns `&''static str`, so the ingest path cannot produce a null — the column being nullable let stale fixtures hold one, which then broke a repair query that legitimately assumed the contract.';
comment on column transcript_turns.skill
     is 'Which skill/plugin drove the turn — the "are our skills used?" question, the same shape as the unused-tools signal but for skills.';
comment on column transcript_turns.attrs
     is 'Every per-turn attribute the transcript carried, verbatim, including the fields not promoted to columns. Keeps a new signal a query rather than a re-ingest of files the user may have rotated away. See docs/database/activity.md.';
