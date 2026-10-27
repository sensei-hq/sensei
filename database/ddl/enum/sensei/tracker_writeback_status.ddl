set search_path to sensei, extensions;

-- A ticket's last usage write-back outcome (spec tracker-metrics-writeback W9).
-- Order and labels are pinned to `crate::tracker::writeback::WritebackStatus::ALL` by a test.
create type tracker_writeback_status as enum (
  'ok'               -- the last write landed
, 'forbidden'        -- 401/403: the token may not edit the ticket or its comments (sticky)
, 'field_invalid'    -- 400 on a mapped-field write (sticky until the mapping changes)
, 'comment_removed'  -- someone deleted sensei's comment (sticky: sensei does not re-post)
, 'not_found'        -- the ticket is gone (sticky)
, 'rate_limited'     -- 429 after the retry budget (retried next pass)
, 'error'            -- anything else (retried next pass)
);

comment on type tracker_writeback_status is
'Last write-back outcome of a sensei.tracker_tasks row (W9). Mirrors crate::tracker::writeback::WritebackStatus.';
