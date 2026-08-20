set search_path to activity, sensei, extensions;

-- Repo-grain tool usage, mapped from the RAW assistant event stream — the replacement
-- for the retired `unused_tools` snapshot metric. Each tool-call event (PostToolUse,
-- the completed call that carries the outcome) is attributed to a repository by
-- resolving its `cwd` through the same folder→repository lookup the rest of the system
-- uses: `sensei.repo_anchor_for(cwd)` → the nearest repo-kind ancestor folder →
-- `folders.repository_id`. Rolled up per (repository, family, tool, day) with call /
-- success / failure / session counts, so tool usage is a real per-day series (not a
-- today-only snapshot) grounded in every actual invocation (never the sparse,
-- lagging `tool_call_verdicts` classifier layer).
--
-- Honest mapping: an event whose `cwd` resolves to no repo-anchored folder (a path
-- outside any tracked checkout) contributes NO row — it is never fabricated into a
-- repository. `cwd`-null events (older captures) likewise drop out until they carry a
-- working directory.
--
-- NOTE (perf): `repo_anchor_for(cwd)` runs per row, so a full scan of this view walks
-- the whole event history. Callers filter (by repository, or a recent window). The
-- planned optimization is a stored `assistant_events.repository_id` column derived
-- from `cwd` at capture (+ a backfill), after which this view reads the column
-- directly instead of resolving per row.
create or replace view tool_usage_by_repository as
select f.repository_id
     , ae.family
     , ae.tool_name
     , date_trunc('day', ae.created_at)::date          as day
     , count(*)                                        as calls
     , count(*) filter (where ae.success is true)      as succeeded
     , count(*) filter (where ae.success is false)     as failed
     , count(distinct ae.session_id)                   as sessions
  from activity.assistant_events ae
  cross join lateral sensei.repo_anchor_for(ae.cwd) ra
  join sensei.folders f on f.id = ra.repo_folder_id
 where ae.event_type   = 'PostToolUse'   -- the completed call (carries the outcome); avoids double-counting the Pre/Post pair
   and ae.tool_name   is not null
   and ae.cwd         is not null
   and f.repository_id is not null
 group by f.repository_id, ae.family, ae.tool_name, day;

comment on view tool_usage_by_repository is
'Repo-grain tool usage from the raw assistant event stream: each PostToolUse event
mapped to a repository via repo_anchor_for(cwd) -> folders.repository_id, rolled up
per (repository, family, tool, day). Replaces the retired unused_tools snapshot.';
