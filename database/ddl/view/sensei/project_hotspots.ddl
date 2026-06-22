set search_path to sensei, activity, extensions;

-- Files with high edit / rework frequency per project, derived from the
-- assistant event stream (the former activity.events source was never
-- populated). edit_count = Edit/Write/MultiEdit tool calls touching the file;
-- correction_count = RE-EDITS (edits beyond the first per session = edit_count -
-- distinct sessions) — the rework signal, since tool failures aren't captured.
-- 30-day window. Same column shape as before (read by get_hotspots).
CREATE OR REPLACE VIEW sensei.project_hotspots AS
SELECT s.project_id
     , f.name                                                              AS folder
     , ae.payload->'tool_input'->>'file_path'                              AS file_path
     , COUNT(*)                                                            AS edit_count
     , (COUNT(*) - COUNT(DISTINCT ae.session_id))                          AS correction_count
     , to_timestamp(MAX(ae.ts) / 1000.0)                                   AS last_event_at
  FROM activity.assistant_events ae
  JOIN activity.sessions s ON s.client_session_id = ae.session_id
  JOIN sensei.folders    f ON f.id = s.folder_id
 WHERE ae.event_type = 'PostToolUse'
   AND ae.tool_name IN ('Edit', 'Write', 'MultiEdit')
   AND ae.payload->'tool_input' ? 'file_path'
   AND to_timestamp(ae.ts / 1000.0) > now() - interval '30 days'
 GROUP BY s.project_id, f.name, ae.payload->'tool_input'->>'file_path';

comment on view project_hotspots is
'Files with high edit/rework frequency per project, from activity.assistant_events.
Powers the Hotspots section in project overview and the rework graph overlay.
- edit_count: Edit/Write/MultiEdit tool calls touching this file (30-day window)
- correction_count: re-edits = edit_count - distinct sessions (edits beyond the
  first per session) — the rework signal, since tool failures aren''t captured
- Order by (correction_count + edit_count) DESC for rework ranking';
