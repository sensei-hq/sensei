set search_path to activity, sensei, extensions;

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
  , origin         text not null default 'human'  -- who started the turn: human, or Claude Code's scheduled / peer / task_notification / auto_continuation
  , prompt_id      text     -- the transcript's promptId — the hook's prompt_id for the same prompt
  , intent         sensei.turn_intent         -- #447: what this user turn means for the reply it answers
  , intent_source  sensei.turn_intent_source  -- #447: who decided it — the model, or the phrase fallback
  , intent_at      timestamptz                -- #447: when it was last (re)labelled; a later one makes the session due again
  , constraint transcript_turns_intent_has_source check ((intent is null) = (intent_source is null) and (intent is null) = (intent_at is null))
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

comment on column transcript_turns.intent
     is 'What this user turn means for the agent reply before it (#447): correction, principle or neither. NULL until labelled; a re-ingest that changes user_text clears it.';
comment on column transcript_turns.intent_source
     is 'Who decided `intent` (#447): intent = the measured classifier (qwen3.8, two stages); phrase = the key-phrase fallback when the model was unavailable, relabelled when it returns.';
comment on column transcript_turns.intent_at
     is 'When `intent` was last written (#447). A label newer than the session''s analyzed_at makes the session due for re-analysis.';
