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
  -- NOT NULL: `TranscriptAdapter::family()` returns `&'static str`, so the ingest
  -- path cannot produce a null — the column being nullable let stale fixtures hold
  -- one, which then broke a repair query that legitimately assumed the contract.
  , family         text not null
  , provider       text
  , model          text
  , turn_index     integer not null
  , user_text      text
  , assistant_text text
  , char_count     integer not null default 0
  , started_at     timestamptz
  , created_at     timestamptz not null default now()
  -- Every per-turn attribute the transcript carried, verbatim. The adapters see a
  -- far richer record than we model (parentUuid, requestId, permissionMode,
  -- usage.speed, usage.server_tool_use, …) and anything not promoted below used to
  -- be dropped on the floor at parse time — unrecoverable without re-reading files
  -- the user may have rotated away. Keep the raw shape here so a new signal is a
  -- query, not a re-ingest, and promote a column only once something reads it.
  , attrs          jsonb not null default '{}'::jsonb
  -- ── Promoted: token accounting ────────────────────────────────────────────
  -- Split, NOT summed. `tokens_in` on activity.sessions folds fresh input +
  -- cache-write + cache-read into one number, and measured against real
  -- transcripts ~98% of it is cache reads — which bill about 10x cheaper. Every
  -- cost metric built on that sum therefore reads roughly an order of magnitude
  -- high, and improving cache use makes it go UP. Kept separate at this grain so
  -- cost can be computed honestly.
  , tokens_in      bigint   -- fresh input only (`input_tokens`)
  , tokens_out     bigint
  , cache_read     bigint   -- `cache_read_input_tokens`
  , cache_write    bigint   -- `cache_creation_input_tokens`
  -- ── Promoted: signals with a known consumer ───────────────────────────────
  -- `max_tokens` is a DETERMINISTIC context-pressure signal; the shipped
  -- context_pressure_rate metric currently infers it from a text hint.
  , stop_reason    text
  -- Subagent work is merged into the main thread today, so its cost is invisible.
  , is_sidechain   boolean
  -- Which skill/plugin drove the turn — the "are our skills used?" question, the
  -- same shape as the unused-tools signal but for skills.
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
