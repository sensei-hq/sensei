set search_path to activity, extensions;

-- One row per API response in a transcript, its usage counted ONCE
-- (docs/spec/daemon/session-api-cost.md C1/C2). Claude Code writes a response on
-- one line per content block, each repeating its usage; this is the response.
-- Facts, not prices: activity.message_api_cost prices a row at read, so a price
-- change never needs a re-read. Written at transcript ingest; a session's rows
-- are replaced together.
create table if not exists transcript_messages (
  source            text        not null
, session_id        text        not null
, message_id        text        not null
, turn_index        integer
, sent_at           timestamptz
, model             text        not null
, speed             text
, inference_geo     text
, service_tier      text
, input             bigint      not null
, cache_write_5m    bigint      not null
, cache_write_1h    bigint      not null
, cache_split_known boolean     not null
, cache_read        bigint      not null
, output            bigint      not null
, thinking          bigint
, primary key (source, session_id, message_id)
);

create index if not exists transcript_messages_session_idx
    on transcript_messages (session_id);

comment on table transcript_messages is
'One row per API response in a transcript, usage counted once; priced at read by activity.message_api_cost.';
comment on column transcript_messages.message_id is
'message.id, or line:<n> for a usage record that carries none.';
comment on column transcript_messages.turn_index is
'The transcript turn the response falls in; NULL before the first human prompt.';
comment on column transcript_messages.input is
'Fresh input tokens only (usage.input_tokens).';
comment on column transcript_messages.cache_write_5m is
'Cache writes to the 5-minute cache; holds the whole cache write when the split is unknown.';
comment on column transcript_messages.cache_split_known is
'false when the record gave only cache_creation_input_tokens, so cache_write_5m holds the total.';
comment on column transcript_messages.output is
'Output tokens, thinking included.';
comment on column transcript_messages.thinking is
'The thinking part of output (output_tokens_details.thinking_tokens); never added to output.';
