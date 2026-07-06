set search_path to sensei, extensions;

-- #90 Per-tool-call usage verdict — did the assistant actually consume the
-- response, or did it skip past it? Joined at read time to the Replay tab's
-- timeline so an idempotent recompute never touches activity data.
--
-- Option B from the ticket. Preferred over a `usage_verdict` column on
-- `activity.assistant_events` because:
--   * The classifier can be re-run against evolved heuristics without
--     touching the source-of-truth event stream.
--   * `activity.assistant_events` is a firehose (~95k rows for one sensei
--     install); a nullable enum column would bloat every row for a signal
--     only Replay consumes.
--
-- Verdict is derived from the pair (PostToolUse event, next event on the
-- same session):
--   * 'used'    — next PreToolUse's tool_input cites fragments of this
--                  PostToolUse's tool_response (path, symbol, snippet).
--   * 'partial' — next event is a tool call on the same target (same file
--                  path / same URL) but doesn't cite the response text.
--   * 'ignored' — next event is UserPromptSubmit / Stop / unrelated tool
--                  call → the assistant didn't build on this response.
create table if not exists tool_call_verdicts (
  id             bigserial   primary key
, session_id     text        not null
, event_id       bigint      not null unique   -- FK-less by design: assistant_events lives in activity schema and rotates on prune; verdicts orphan naturally
, tool_name      text
, verdict        text        not null check (verdict in ('used', 'partial', 'ignored'))
, confidence     real        not null default 1.0
, reason         text
, classified_at  timestamptz not null default now()
);

create index if not exists tool_call_verdicts_session_idx on tool_call_verdicts(session_id);
create index if not exists tool_call_verdicts_event_idx   on tool_call_verdicts(event_id);
create index if not exists tool_call_verdicts_verdict_idx on tool_call_verdicts(verdict);

comment on table tool_call_verdicts is
'#90 Per-tool-call usage verdict (used / partial / ignored). Classified from
the (PostToolUse, next-event) pair on the same session. Rebuildable at any
time — the classifier is idempotent, so re-running against a new heuristic
just refreshes the table. Read-time joined by the Replay tab (#84).';
